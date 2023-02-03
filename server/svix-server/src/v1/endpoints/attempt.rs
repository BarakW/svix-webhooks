// SPDX-FileCopyrightText: © 2022 Svix Authors
// SPDX-License-Identifier: MIT

use crate::{
    core::{
        permissions,
        types::{
            EndpointId, EndpointIdOrUid, EventChannel, EventTypeNameSet, MessageAttemptId,
            MessageAttemptTriggerType, MessageEndpointId, MessageId, MessageStatus,
            StatusCodeClass,
        },
    },
    ctx,
    db::models::{endpoint, message, messagedestination},
    err_database,
    error::{Error, HttpError, Result},
    queue::MessageTask,
    v1::{
        endpoints::message::MessageOut,
        utils::{
            apply_pagination, iterator_from_before_or_after, openapi_tag, ApplicationEndpointPath,
            ApplicationMsgAttemptPath, ApplicationMsgEndpointPath, ApplicationMsgPath,
            EmptyResponse, ListResponse, ModelOut, PaginationLimit, ReversibleIterator,
            ValidatedQuery,
        },
    },
    AppState,
};
use aide::axum::{
    routing::{delete, get, post},
    ApiRouter,
};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};

use hyper::StatusCode;
use schemars::JsonSchema;
use sea_orm::{
    entity::prelude::*, sea_query::Expr, DatabaseConnection, IntoActiveModel, QueryOrder,
    QuerySelect,
};
use serde::{Deserialize, Serialize};

use svix_server_derive::ModelOut;
use validator::Validate;

use crate::db::models::messageattempt;
use crate::v1::utils::Pagination;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ModelOut, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageAttemptOut {
    pub url: String,
    pub response: String,
    pub response_status_code: i16,
    pub status: MessageStatus,
    pub trigger_type: MessageAttemptTriggerType,
    pub msg_id: MessageId,
    pub endpoint_id: EndpointId,

    pub id: MessageAttemptId,

    #[serde(rename = "timestamp")]
    pub created_at: DateTime<Utc>,
}

// FIXME: This can and should be a derive macro
impl From<messageattempt::Model> for MessageAttemptOut {
    fn from(model: messageattempt::Model) -> Self {
        Self {
            url: model.url,
            response: model.response,
            response_status_code: model.response_status_code,
            status: model.status,
            trigger_type: model.trigger_type,
            msg_id: model.msg_id,
            endpoint_id: model.endp_id,

            id: model.id,
            created_at: model.created_at.into(),
        }
    }
}

/// A model containing information on a given message plus additional fields on the last attempt for
/// that message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttemptedMessageOut {
    #[serde(flatten)]
    pub msg: MessageOut,
    pub status: MessageStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_attempt: Option<DateTimeWithTimeZone>,
}

impl ModelOut for AttemptedMessageOut {
    fn id_copy(&self) -> String {
        self.msg.id.0.clone()
    }
}

impl AttemptedMessageOut {
    pub fn from_dest_and_msg(
        dest: messagedestination::Model,
        msg: message::Model,
    ) -> AttemptedMessageOut {
        AttemptedMessageOut {
            msg: msg.into(),
            status: dest.status,
            next_attempt: dest.next_attempt,
        }
    }
}

/// Additional parameters (besides pagination) in the query string for the "List Attempted Messages"
/// endpoint.
#[derive(Debug, Deserialize, Validate, JsonSchema)]
pub struct ListAttemptedMessagesQueryParameters {
    #[validate]
    channel: Option<EventChannel>,
    status: Option<MessageStatus>,
    before: Option<DateTime<Utc>>,
    after: Option<DateTime<Utc>>,
}

/// Fetches a list of [`AttemptedMessageOut`]s associated with a given app and endpoint.
async fn list_attempted_messages(
    State(AppState { ref db, .. }): State<AppState>,
    ValidatedQuery(pagination): ValidatedQuery<Pagination<ReversibleIterator<MessageId>>>,
    ValidatedQuery(ListAttemptedMessagesQueryParameters {
        channel,
        status,
        before,
        after,
    }): ValidatedQuery<ListAttemptedMessagesQueryParameters>,
    Path(ApplicationEndpointPath { endpoint_id, .. }): Path<ApplicationEndpointPath>,
    permissions::Application { app }: permissions::Application,
) -> Result<Json<ListResponse<AttemptedMessageOut>>> {
    let PaginationLimit(limit) = pagination.limit;
    let endp = ctx!(
        endpoint::Entity::secure_find_by_id_or_uid(app.id.clone(), endpoint_id)
            .one(db)
            .await
    )?
    .ok_or_else(|| HttpError::not_found(None, None))?;

    let mut dests_and_msgs = messagedestination::Entity::secure_find_by_endpoint(endp.id)
        .find_also_related(message::Entity);

    if let Some(channel) = channel {
        dests_and_msgs =
            dests_and_msgs.filter(Expr::cust_with_values("channels ?? ?", vec![channel]));
    }

    if let Some(status) = status {
        dests_and_msgs = dests_and_msgs.filter(messagedestination::Column::Status.eq(status));
    }

    async fn _get_msg_dest_id(
        db: &DatabaseConnection,
        msg_id: MessageId,
    ) -> Result<MessageEndpointId> {
        Ok(ctx!(
            messagedestination::Entity::secure_find_by_msg(msg_id)
                .one(db)
                .await
        )?
        .ok_or_else(|| HttpError::bad_request(None, Some("Invalid iterator".to_owned())))?
        .id)
    }

    let msg_dest_iterator = match pagination.iterator {
        Some(ReversibleIterator::Normal(msg_id)) => Some(ReversibleIterator::Normal(
            _get_msg_dest_id(db, msg_id).await?,
        )),
        Some(ReversibleIterator::Prev(msg_id)) => Some(ReversibleIterator::Prev(
            _get_msg_dest_id(db, msg_id).await?,
        )),
        None => None,
    };
    let iterator = iterator_from_before_or_after(msg_dest_iterator, before, after);
    let is_prev = matches!(iterator, Some(ReversibleIterator::Prev(_)));

    let dests_and_msgs = apply_pagination(
        dests_and_msgs,
        messagedestination::Column::Id,
        limit,
        iterator,
    );

    let into = |(dest, msg): (messagedestination::Model, Option<message::Model>)| {
        let msg =
            msg.ok_or_else(|| err_database!("No associated message with messagedestination"))?;
        Ok(AttemptedMessageOut::from_dest_and_msg(dest, msg))
    };

    let out = if is_prev {
        ctx!(dests_and_msgs.all(db).await)?
            .into_iter()
            .rev()
            .map(into)
            .collect::<Result<_>>()?
    } else {
        ctx!(dests_and_msgs.all(db).await)?
            .into_iter()
            .map(into)
            .collect::<Result<_>>()?
    };

    Ok(Json(AttemptedMessageOut::list_response(
        out,
        limit as usize,
        is_prev,
    )))
}

/// Additional parameters (besides pagination) in the query string for the "List Attempts by
/// Endpoint" endpoint.
#[derive(Debug, Deserialize, Validate, JsonSchema)]
pub struct ListAttemptsByEndpointQueryParameters {
    status: Option<MessageStatus>,
    status_code_class: Option<StatusCodeClass>,
    #[validate]
    event_types: Option<EventTypeNameSet>,
    #[validate]
    channel: Option<EventChannel>,
    before: Option<DateTime<Utc>>,
    after: Option<DateTime<Utc>>,
}

// Applies filters common to [`list_attempts_by_endpoint`] and [`list_attempts_by_msg`]
fn list_attempts_by_endpoint_or_message_filters(
    mut query: Select<messageattempt::Entity>,
    status: Option<MessageStatus>,
    status_code_class: Option<StatusCodeClass>,
    event_types: Option<EventTypeNameSet>,
    channel: Option<EventChannel>,
) -> Select<messageattempt::Entity> {
    if let Some(status) = status {
        query = query.filter(messageattempt::Column::Status.eq(status));
    }

    query = match status_code_class {
        Some(StatusCodeClass::CodeNone) => {
            query.filter(messageattempt::Column::ResponseStatusCode.between(0, 99))
        }

        Some(StatusCodeClass::Code1xx) => {
            query.filter(messageattempt::Column::ResponseStatusCode.between(100, 199))
        }

        Some(StatusCodeClass::Code2xx) => {
            query.filter(messageattempt::Column::ResponseStatusCode.between(200, 299))
        }

        Some(StatusCodeClass::Code3xx) => {
            query.filter(messageattempt::Column::ResponseStatusCode.between(300, 399))
        }

        Some(StatusCodeClass::Code4xx) => {
            query.filter(messageattempt::Column::ResponseStatusCode.between(400, 499))
        }

        Some(StatusCodeClass::Code5xx) => {
            query.filter(messageattempt::Column::ResponseStatusCode.between(500, 599))
        }

        None => query,
    };

    // The event_types and channel filter require joining the associated message
    if event_types.is_some() || channel.is_some() {
        query = query.join_rev(
            sea_orm::JoinType::InnerJoin,
            messageattempt::Entity::belongs_to(message::Entity)
                .from(messageattempt::Column::MsgId)
                .to(message::Column::Id)
                .into(),
        );

        if let Some(EventTypeNameSet(event_types)) = event_types {
            query = query.filter(message::Column::EventType.is_in(event_types));
        }

        if let Some(channel) = channel {
            query = query.filter(Expr::cust_with_values("channels ?? ?", vec![channel]));
        }
    }

    query
}

/// Fetches a list of [`MessageAttemptOut`]s for a given endpoint ID
async fn list_attempts_by_endpoint(
    State(AppState { ref db, .. }): State<AppState>,
    ValidatedQuery(pagination): ValidatedQuery<Pagination<ReversibleIterator<MessageAttemptId>>>,
    ValidatedQuery(ListAttemptsByEndpointQueryParameters {
        status,
        status_code_class,
        event_types,
        channel,
        before,
        after,
    }): ValidatedQuery<ListAttemptsByEndpointQueryParameters>,
    Path(ApplicationEndpointPath { endpoint_id, .. }): Path<ApplicationEndpointPath>,
    permissions::Application { app }: permissions::Application,
) -> Result<Json<ListResponse<MessageAttemptOut>>> {
    let PaginationLimit(limit) = pagination.limit;
    // Confirm endpoint ID belongs to the given application
    let endp = ctx!(
        endpoint::Entity::secure_find_by_id_or_uid(app.id.clone(), endpoint_id)
            .one(db)
            .await
    )?
    .ok_or_else(|| HttpError::not_found(None, None))?;

    let query = list_attempts_by_endpoint_or_message_filters(
        messageattempt::Entity::secure_find_by_endpoint(endp.id),
        status,
        status_code_class,
        event_types,
        channel,
    );

    let iterator = iterator_from_before_or_after(pagination.iterator, before, after);
    let is_prev = matches!(iterator, Some(ReversibleIterator::Prev(_)));
    let query = apply_pagination(query, messageattempt::Column::Id, limit, iterator);

    let out = if is_prev {
        ctx!(query.all(db).await)?
            .into_iter()
            .rev()
            .map(Into::into)
            .collect()
    } else {
        ctx!(query.all(db).await)?
            .into_iter()
            .map(Into::into)
            .collect()
    };

    Ok(Json(MessageAttemptOut::list_response(
        out,
        limit as usize,
        is_prev,
    )))
}

/// Flattens in a [`ListAttemptsByEndpointOrMsgQueryParameters`] and adds one extra query parameter
#[derive(Debug, Deserialize, Validate, JsonSchema)]
pub struct ListAttemptsByMsgQueryParameters {
    status: Option<MessageStatus>,
    status_code_class: Option<StatusCodeClass>,
    #[validate]
    event_types: Option<EventTypeNameSet>,
    #[validate]
    channel: Option<EventChannel>,
    #[validate]
    endpoint_id: Option<EndpointIdOrUid>,
    before: Option<DateTime<Utc>>,
    after: Option<DateTime<Utc>>,
}

/// Fetches a list of [`MessageAttemptOut`]s for a given message ID
async fn list_attempts_by_msg(
    State(AppState { ref db, .. }): State<AppState>,
    ValidatedQuery(pagination): ValidatedQuery<Pagination<ReversibleIterator<MessageAttemptId>>>,
    ValidatedQuery(ListAttemptsByMsgQueryParameters {
        status,
        status_code_class,
        event_types,
        channel,
        endpoint_id,
        before,
        after,
    }): ValidatedQuery<ListAttemptsByMsgQueryParameters>,
    Path(ApplicationMsgPath { msg_id, .. }): Path<ApplicationMsgPath>,
    permissions::Application { app }: permissions::Application,
) -> Result<Json<ListResponse<MessageAttemptOut>>> {
    let PaginationLimit(limit) = pagination.limit;
    // Confirm message ID belongs to the given application
    let msg = ctx!(
        message::Entity::secure_find_by_id_or_uid(app.id.clone(), msg_id)
            .one(db)
            .await
    )?
    .ok_or_else(|| HttpError::not_found(None, None))?;

    let mut query = list_attempts_by_endpoint_or_message_filters(
        messageattempt::Entity::secure_find_by_msg(msg.id),
        status,
        status_code_class,
        event_types,
        channel,
    );

    if let Some(endpoint_id) = endpoint_id {
        // Ensure the endpoint ID/UID belongs to the given application
        if let Some(endp) = ctx!(
            endpoint::Entity::secure_find_by_id_or_uid(app.id, endpoint_id)
                .one(db)
                .await
        )? {
            // And filter by its ID incase a UID was used
            query = query.filter(messageattempt::Column::EndpId.eq(endp.id));
        } else {
            return Err(Error::http(HttpError::not_found(None, None)));
        }
    }

    let iterator = iterator_from_before_or_after(pagination.iterator, before, after);
    let is_prev = matches!(iterator, Some(ReversibleIterator::Prev(_)));
    let query = apply_pagination(query, messageattempt::Column::Id, limit, iterator);
    let out = if is_prev {
        ctx!(query.all(db).await)?
            .into_iter()
            .rev()
            .map(Into::into)
            .collect()
    } else {
        ctx!(query.all(db).await)?
            .into_iter()
            .map(Into::into)
            .collect()
    };

    Ok(Json(MessageAttemptOut::list_response(
        out,
        limit as usize,
        is_prev,
    )))
}

/// A type combining information from [`messagedestination::Model`]s and [`endpoint::Model`]s to
/// output information on attempted destinations
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageEndpointOut {
    #[serde(flatten)]
    endpoint: super::endpoint::EndpointOutCommon,
    id: EndpointId,
    status: MessageStatus,
    next_attempt: Option<DateTime<Utc>>,
}

impl ModelOut for MessageEndpointOut {
    fn id_copy(&self) -> String {
        self.id.0.clone()
    }
}

impl MessageEndpointOut {
    fn from_dest_and_endp(dest: messagedestination::Model, endp: endpoint::Model) -> Self {
        MessageEndpointOut {
            id: endp.id.clone(),
            endpoint: endp.into(),
            status: dest.status,
            next_attempt: dest.next_attempt.map(Into::into),
        }
    }
}

async fn list_attempted_destinations(
    State(AppState { ref db, .. }): State<AppState>,
    ValidatedQuery(mut pagination): ValidatedQuery<Pagination<EndpointId>>,
    Path(ApplicationMsgPath { msg_id, .. }): Path<ApplicationMsgPath>,
    permissions::Application { app }: permissions::Application,
) -> Result<Json<ListResponse<MessageEndpointOut>>> {
    let PaginationLimit(limit) = pagination.limit;
    let iterator = pagination.iterator.take();

    // Confirm message ID belongs to the given application while fetching the ID in case a UID was
    // given
    let msg_id = if let Some(message) = ctx!(
        message::Entity::secure_find_by_id_or_uid(app.id.clone(), msg_id.clone())
            .one(db)
            .await
    )? {
        message.id
    } else {
        return Err(Error::http(HttpError::not_found(None, None)));
    };

    // Fetch the [`messagedestination::Model`] and associated [`endpoint::Model`]
    let mut query = messagedestination::Entity::secure_find_by_msg(msg_id)
        .find_also_related(endpoint::Entity)
        .order_by_desc(messagedestination::Column::EndpId)
        .limit(limit + 1);

    if let Some(iterator) = iterator {
        query = query.filter(messagedestination::Column::EndpId.lt(iterator));
    }

    Ok(Json(MessageEndpointOut::list_response_no_prev(
        ctx!(query.all(db).await)?
            .into_iter()
            .map(
                |(dest, endp): (messagedestination::Model, Option<endpoint::Model>)| {
                    let endp = endp.ok_or_else(|| {
                        err_database!("No associated endpoint with messagedestination")
                    })?;
                    Ok(MessageEndpointOut::from_dest_and_endp(dest, endp))
                },
            )
            .collect::<Result<_>>()?,
        limit as usize,
    )))
}

#[derive(Debug, Deserialize, Validate, JsonSchema)]
pub struct ListAttemptsForEndpointQueryParameters {
    #[validate]
    pub channel: Option<EventChannel>,
    pub event_types: Option<EventTypeNameSet>,
    pub status: Option<MessageStatus>,
    pub before: Option<DateTime<Utc>>,
    pub after: Option<DateTime<Utc>>,
}

async fn list_attempts_for_endpoint(
    state: State<AppState>,
    pagination: ValidatedQuery<Pagination<ReversibleIterator<MessageAttemptId>>>,
    ValidatedQuery(ListAttemptsForEndpointQueryParameters {
        channel,
        event_types,
        status,
        before,
        after,
    }): ValidatedQuery<ListAttemptsForEndpointQueryParameters>,

    Path(ApplicationMsgEndpointPath {
        app_id,
        msg_id,
        endpoint_id,
    }): Path<ApplicationMsgEndpointPath>,

    auth_app: permissions::Application,
) -> Result<Json<ListResponse<MessageAttemptOut>>> {
    list_messageattempts(
        state,
        pagination,
        ValidatedQuery(AttemptListFetchOptions {
            endpoint_id: Some(endpoint_id),
            channel,
            event_types,
            status,
            before,
            after,
        }),
        Path(ApplicationMsgPath { app_id, msg_id }),
        auth_app,
    )
    .await
}

#[derive(Debug, Deserialize, Validate, JsonSchema)]
pub struct AttemptListFetchOptions {
    #[validate]
    pub endpoint_id: Option<EndpointIdOrUid>,
    #[validate]
    pub channel: Option<EventChannel>,
    pub event_types: Option<EventTypeNameSet>,
    pub status: Option<MessageStatus>,
    pub before: Option<DateTime<Utc>>,
    pub after: Option<DateTime<Utc>>,
}

async fn list_messageattempts(
    State(AppState { ref db, .. }): State<AppState>,
    ValidatedQuery(pagination): ValidatedQuery<Pagination<ReversibleIterator<MessageAttemptId>>>,
    ValidatedQuery(AttemptListFetchOptions {
        endpoint_id,
        channel,
        event_types,
        status,
        before,
        after,
    }): ValidatedQuery<AttemptListFetchOptions>,
    Path(ApplicationMsgPath { msg_id, .. }): Path<ApplicationMsgPath>,

    permissions::Application { app }: permissions::Application,
) -> Result<Json<ListResponse<MessageAttemptOut>>> {
    let PaginationLimit(limit) = pagination.limit;
    let msg = ctx!(
        message::Entity::secure_find_by_id_or_uid(app.id.clone(), msg_id)
            .one(db)
            .await
    )?
    .ok_or_else(|| HttpError::not_found(None, None))?;

    let mut query = messageattempt::Entity::secure_find_by_msg(msg.id);

    if let Some(endpoint_id) = endpoint_id {
        let endp = ctx!(
            endpoint::Entity::secure_find_by_id_or_uid(app.id.clone(), endpoint_id)
                .one(db)
                .await
        )?
        .ok_or_else(|| HttpError::not_found(None, None))?;
        query = query.filter(messageattempt::Column::EndpId.eq(endp.id))
    }

    if let Some(status) = status {
        query = query.filter(messageattempt::Column::Status.eq(status))
    }

    if let Some(channel) = channel {
        query = query.filter(Expr::cust_with_values("channels ?? ?", vec![channel]));
    }

    if let Some(EventTypeNameSet(event_types)) = event_types {
        query = query.filter(message::Column::EventType.is_in(event_types));
    }

    let iterator = iterator_from_before_or_after(pagination.iterator, before, after);
    let is_prev = matches!(iterator, Some(ReversibleIterator::Prev(_)));
    let query = apply_pagination(query, messageattempt::Column::Id, limit, iterator);
    let out = if is_prev {
        ctx!(query.all(db).await)?
            .into_iter()
            .rev()
            .map(Into::into)
            .collect()
    } else {
        ctx!(query.all(db).await)?
            .into_iter()
            .map(Into::into)
            .collect()
    };

    Ok(Json(MessageAttemptOut::list_response(
        out,
        limit as usize,
        false,
    )))
}

async fn get_messageattempt(
    State(AppState { ref db, .. }): State<AppState>,
    Path(ApplicationMsgAttemptPath {
        msg_id, attempt_id, ..
    }): Path<ApplicationMsgAttemptPath>,
    permissions::Application { app }: permissions::Application,
) -> Result<Json<MessageAttemptOut>> {
    let msg = ctx!(
        message::Entity::secure_find_by_id_or_uid(app.id, msg_id)
            .one(db)
            .await
    )?
    .ok_or_else(|| HttpError::not_found(None, None))?;

    let attempt = ctx!(
        messageattempt::Entity::secure_find_by_msg(msg.id)
            .filter(messageattempt::Column::Id.eq(attempt_id))
            .one(db)
            .await
    )?
    .ok_or_else(|| HttpError::not_found(None, None))?;
    Ok(Json(attempt.into()))
}

async fn resend_webhook(
    State(AppState {
        ref db, queue_tx, ..
    }): State<AppState>,
    Path(ApplicationMsgEndpointPath {
        msg_id,
        endpoint_id,
        ..
    }): Path<ApplicationMsgEndpointPath>,
    permissions::Application { app }: permissions::Application,
) -> Result<(StatusCode, Json<EmptyResponse>)> {
    let msg = ctx!(
        message::Entity::secure_find_by_id_or_uid(app.id.clone(), msg_id)
            .one(db)
            .await
    )?
    .ok_or_else(|| HttpError::not_found(None, None))?;

    if msg.payload.is_none() {
        return Err(HttpError::bad_request(
            Some("missing_payload".to_string()),
            Some("Unable to resend message. Payload is missing (probably expired).".to_string()),
        )
        .into());
    }

    let endp = ctx!(
        endpoint::Entity::secure_find_by_id_or_uid(app.id.clone(), endpoint_id)
            .one(db)
            .await
    )?
    .ok_or_else(|| HttpError::not_found(None, None))?;

    // Fetch it to make sure it was even a combination
    let _msg_dest = ctx!(
        messagedestination::Entity::secure_find_by_msg(msg.id.clone())
            .filter(messagedestination::Column::EndpId.eq(endp.id.clone()))
            .one(db)
            .await
    )?
    .ok_or_else(|| HttpError::not_found(None, None))?;

    queue_tx
        .send(
            MessageTask::new_task(
                msg.id.clone(),
                app.id,
                endp.id,
                MessageAttemptTriggerType::Manual,
            ),
            None,
        )
        .await?;
    Ok((StatusCode::ACCEPTED, Json(EmptyResponse {})))
}

async fn expunge_attempt_content(
    State(AppState { ref db, .. }): State<AppState>,
    Path(ApplicationMsgAttemptPath {
        msg_id, attempt_id, ..
    }): Path<ApplicationMsgAttemptPath>,
    permissions::OrganizationWithApplication { app }: permissions::OrganizationWithApplication,
) -> Result<StatusCode> {
    let msg = ctx!(
        message::Entity::secure_find_by_id_or_uid(app.id, msg_id)
            .one(db)
            .await
    )?
    .ok_or_else(|| HttpError::not_found(None, Some("Message not found".to_string())))?;

    let mut attempt = ctx!(
        messageattempt::Entity::secure_find_by_msg(msg.id)
            .filter(messageattempt::Column::Id.eq(attempt_id))
            .one(db)
            .await
    )?
    .ok_or_else(|| HttpError::not_found(None, Some("Message attempt not found".to_string())))?
    .into_active_model();

    attempt.response = sea_orm::Set("EXPUNGED".to_string());
    ctx!(attempt.update(db).await)?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> ApiRouter<AppState> {
    ApiRouter::new()
        // NOTE: [`list_messageattempts`] is deprecated
        .api_route_with(
            "/app/:app_id/msg/:msg_id/attempt/",
            get(list_messageattempts),
            openapi_tag("Message Attempt"),
        )
        .api_route_with(
            "/app/:app_id/msg/:msg_id/attempt/:attempt_id/",
            get(get_messageattempt),
            openapi_tag("Message Attempt"),
        )
        .api_route_with(
            "/app/:app_id/msg/:msg_id/attempt/:attempt_id/content/",
            delete(expunge_attempt_content),
            openapi_tag("Message Attempt"),
        )
        .api_route_with(
            "/app/:app_id/msg/:msg_id/endpoint/",
            get(list_attempted_destinations),
            openapi_tag("Message Attempt"),
        )
        .api_route_with(
            "/app/:app_id/msg/:msg_id/endpoint/:endpoint_id/resend/",
            post(resend_webhook),
            openapi_tag("Message Attempt"),
        )
        // NOTE: [`list_attempts_for_endpoint`] is deprecated
        .api_route_with(
            "/app/:app_id/msg/:msg_id/endpoint/:endpoint_id/attempt/",
            get(list_attempts_for_endpoint),
            openapi_tag("Message Attempt"),
        )
        .api_route_with(
            "/app/:app_id/endpoint/:endpoint_id/msg/",
            get(list_attempted_messages),
            openapi_tag("Message Attempt"),
        )
        .api_route_with(
            "/app/:app_id/attempt/endpoint/:endpoint_id/",
            get(list_attempts_by_endpoint),
            openapi_tag("Message Attempt"),
        )
        .api_route_with(
            "/app/:app_id/attempt/msg/:msg_id/",
            get(list_attempts_by_msg),
            openapi_tag("Message Attempt"),
        )
}

#[cfg(test)]
mod tests {
    use super::{
        AttemptListFetchOptions, ListAttemptedMessagesQueryParameters,
        ListAttemptsByEndpointQueryParameters, ListAttemptsByMsgQueryParameters,
        ListAttemptsForEndpointQueryParameters,
    };
    use serde_json::json;
    use validator::Validate;

    const INVALID_CHANNEL: &str = "$$invalid-channel";
    const VALID_CHANNEL: &str = "valid-channel";
    const INVALID_EVENT_TYPES: &[&str] = &["valid-event-type", "&&invalid-event-type"];
    const VALID_EVENT_TYPES: &[&str] = &["valid-event-type", "another-valid-event-type"];
    const INVALID_ENDPOINT_ID: &str = "$$invalid-endpoint";
    const VALID_ENDPOINT_ID: &str = "ep_valid-endpoint";

    #[test]
    fn test_list_attempted_messages_query_params_validation() {
        let q: ListAttemptedMessagesQueryParameters =
            serde_json::from_value(json!({ "channel": INVALID_CHANNEL })).unwrap();
        assert!(q.validate().is_err());

        let q: ListAttemptedMessagesQueryParameters =
            serde_json::from_value(json!({ "channel": VALID_CHANNEL })).unwrap();
        q.validate().unwrap();
    }

    #[test]
    fn test_list_attempts_by_endpoint_query_parameters_validation() {
        let q: ListAttemptsByEndpointQueryParameters =
            serde_json::from_value(json!({ "event_types": INVALID_EVENT_TYPES })).unwrap();
        assert!(q.validate().is_err());

        let q: ListAttemptsByEndpointQueryParameters =
            serde_json::from_value(json!({ "channel": INVALID_CHANNEL })).unwrap();
        assert!(q.validate().is_err());
    }

    #[test]
    fn test_list_attempts_by_msg_query_parameters_validation() {
        let q: ListAttemptsByMsgQueryParameters =
            serde_json::from_value(json!({ "event_types": INVALID_EVENT_TYPES })).unwrap();
        assert!(q.validate().is_err());

        let q: ListAttemptsByMsgQueryParameters =
            serde_json::from_value(json!({ "channel": INVALID_CHANNEL })).unwrap();
        assert!(q.validate().is_err());

        let q: ListAttemptsByMsgQueryParameters =
            serde_json::from_value(json!({ "endpoint_id": INVALID_ENDPOINT_ID })).unwrap();
        assert!(q.validate().is_err());

        let q: ListAttemptsByMsgQueryParameters = serde_json::from_value(json!(
            {
                "event_types": VALID_EVENT_TYPES,
                "channel": VALID_CHANNEL,
                "endpoint_id": VALID_ENDPOINT_ID
            }
        ))
        .unwrap();
        q.validate().unwrap();
    }

    #[test]
    fn test_list_attempts_for_endpoint_query_parameters_validation() {
        let q: ListAttemptsForEndpointQueryParameters =
            serde_json::from_value(json!({ "channel": INVALID_CHANNEL })).unwrap();
        assert!(q.validate().is_err());

        let q: ListAttemptsForEndpointQueryParameters =
            serde_json::from_value(json!({ "channel": VALID_CHANNEL })).unwrap();
        q.validate().unwrap();
    }

    #[test]
    fn test_attempt_list_fetch_options_validation() {
        let q: AttemptListFetchOptions =
            serde_json::from_value(json!({ "endpoint_id": INVALID_ENDPOINT_ID })).unwrap();
        assert!(q.validate().is_err());

        let q: AttemptListFetchOptions =
            serde_json::from_value(json!({ "channel": INVALID_CHANNEL })).unwrap();
        assert!(q.validate().is_err());

        let q: AttemptListFetchOptions = serde_json::from_value(json!(
            {
                "endpoint_id": VALID_ENDPOINT_ID,
                "channel": VALID_CHANNEL
            }
        ))
        .unwrap();
        q.validate().unwrap();
    }
}
