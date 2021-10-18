<?php
/**
 * WebhookTypes
 *
 * PHP version 7.2
 *
 * @category Class
 * @package  Svix
 * @author   OpenAPI Generator team
 * @link     https://openapi-generator.tech
 */

/**
 * Svix API
 *
 * Welcome to the Svix API documentation!  Useful links: [Homepage](https://www.svix.com) | [Support email](mailto:support+docs@svix.com) | [Blog](https://www.svix.com/blog/) | [Slack Community](https://www.svix.com/slack/)  # Introduction  This is the reference documentation and schemas for the [Svix webhook service](https://www.svix.com) API. For tutorials and other documentation please refer to [the documentation](https://docs.svix.com).  ## Main concepts  In Svix you have four important entities you will be interacting with:  - `messages`: these are the webhooks being sent. They can have contents and a few other properties. - `application`: this is where `messages` are sent to. Usually you want to create one application for each of your users. - `endpoint`: endpoints are the URLs messages will be sent to. Each application can have multiple `endpoints` and each message sent to that application will be sent to all of them (unless they are not subscribed to the sent event type). - `event-type`: event types are identifiers denoting the type of the message being sent. Event types are primarily used to decide which events are sent to which endpoint.   ## Authentication  Get your authentication token (`AUTH_TOKEN`) from the [Svix dashboard](https://dashboard.svix.com) and use it as part of the `Authorization` header as such: `Authorization: Bearer ${AUTH_TOKEN}`.  <SecurityDefinitions />   ## Code samples  The code samples assume you already have the respective libraries installed and you know how to use them. For the latest information on how to do that, please refer to [the documentation](https://docs.svix.com/).   ## Cross-Origin Resource Sharing  This API features Cross-Origin Resource Sharing (CORS) implemented in compliance with [W3C spec](https://www.w3.org/TR/cors/). And that allows cross-domain communication from the browser. All responses have a wildcard same-origin which makes them completely public and accessible to everyone, including any code on any site.
 *
 * The version of the OpenAPI document: 1.4
 * Generated by: https://openapi-generator.tech
 * OpenAPI Generator version: 5.2.0
 */

/**
 * NOTE: This class is auto generated by OpenAPI Generator (https://openapi-generator.tech).
 * https://openapi-generator.tech
 * Do not edit the class manually.
 */

namespace Svix\Svix\Models;

use \ArrayAccess;
use \Svix\ObjectSerializer;

/**
 * WebhookTypes Class Doc Comment
 *
 * @category Class
 * @description All of the webhook types that we support
 * @package  Svix
 * @author   OpenAPI Generator team
 * @link     https://openapi-generator.tech
 * @implements \ArrayAccess<TKey, TValue>
 * @template TKey int|null
 * @template TValue mixed|null
 */
class WebhookTypes implements ModelInterface, ArrayAccess, \JsonSerializable
{
    public const DISCRIMINATOR = null;

    /**
      * The original name of the model.
      *
      * @var string
      */
    protected static $openAPIModelName = 'WebhookTypes';

    /**
      * Array of property to type mappings. Used for (de)serialization
      *
      * @var string[]
      */
    protected static $openAPITypes = [
        'a' => '\Svix\Svix\Models\EndpointDisabledEvent',
        'b' => '\Svix\Svix\Models\EndpointCreatedEvent',
        'c' => '\Svix\Svix\Models\EndpointdUpdatedEvent',
        'd' => '\Svix\Svix\Models\EndpointDeletedEvent',
        'e' => '\Svix\Svix\Models\MessageAttemptExhaustedEvent'
    ];

    /**
      * Array of property to format mappings. Used for (de)serialization
      *
      * @var string[]
      * @phpstan-var array<string, string|null>
      * @psalm-var array<string, string|null>
      */
    protected static $openAPIFormats = [
        'a' => null,
        'b' => null,
        'c' => null,
        'd' => null,
        'e' => null
    ];

    /**
     * Array of property to type mappings. Used for (de)serialization
     *
     * @return array
     */
    public static function openAPITypes()
    {
        return self::$openAPITypes;
    }

    /**
     * Array of property to format mappings. Used for (de)serialization
     *
     * @return array
     */
    public static function openAPIFormats()
    {
        return self::$openAPIFormats;
    }

    /**
     * Array of attributes where the key is the local name,
     * and the value is the original name
     *
     * @var string[]
     */
    protected static $attributeMap = [
        'a' => 'a',
        'b' => 'b',
        'c' => 'c',
        'd' => 'd',
        'e' => 'e'
    ];

    /**
     * Array of attributes to setter functions (for deserialization of responses)
     *
     * @var string[]
     */
    protected static $setters = [
        'a' => 'setA',
        'b' => 'setB',
        'c' => 'setC',
        'd' => 'setD',
        'e' => 'setE'
    ];

    /**
     * Array of attributes to getter functions (for serialization of requests)
     *
     * @var string[]
     */
    protected static $getters = [
        'a' => 'getA',
        'b' => 'getB',
        'c' => 'getC',
        'd' => 'getD',
        'e' => 'getE'
    ];

    /**
     * Array of attributes where the key is the local name,
     * and the value is the original name
     *
     * @return array
     */
    public static function attributeMap()
    {
        return self::$attributeMap;
    }

    /**
     * Array of attributes to setter functions (for deserialization of responses)
     *
     * @return array
     */
    public static function setters()
    {
        return self::$setters;
    }

    /**
     * Array of attributes to getter functions (for serialization of requests)
     *
     * @return array
     */
    public static function getters()
    {
        return self::$getters;
    }

    /**
     * The original name of the model.
     *
     * @return string
     */
    public function getModelName()
    {
        return self::$openAPIModelName;
    }


    /**
     * Associative array for storing property values
     *
     * @var mixed[]
     */
    protected $container = [];

    /**
     * Constructor
     *
     * @param mixed[] $data Associated array of property values
     *                      initializing the model
     */
    public function __construct(array $data = null)
    {
        $this->container['a'] = $data['a'] ?? null;
        $this->container['b'] = $data['b'] ?? null;
        $this->container['c'] = $data['c'] ?? null;
        $this->container['d'] = $data['d'] ?? null;
        $this->container['e'] = $data['e'] ?? null;
    }

    /**
     * Show all the invalid properties with reasons.
     *
     * @return array invalid properties with reasons
     */
    public function listInvalidProperties()
    {
        $invalidProperties = [];

        if ($this->container['a'] === null) {
            $invalidProperties[] = "'a' can't be null";
        }
        if ($this->container['b'] === null) {
            $invalidProperties[] = "'b' can't be null";
        }
        if ($this->container['c'] === null) {
            $invalidProperties[] = "'c' can't be null";
        }
        if ($this->container['d'] === null) {
            $invalidProperties[] = "'d' can't be null";
        }
        if ($this->container['e'] === null) {
            $invalidProperties[] = "'e' can't be null";
        }
        return $invalidProperties;
    }

    /**
     * Validate all the properties in the model
     * return true if all passed
     *
     * @return bool True if all properties are valid
     */
    public function valid()
    {
        return count($this->listInvalidProperties()) === 0;
    }


    /**
     * Gets a
     *
     * @return \Svix\Svix\Models\EndpointDisabledEvent
     */
    public function getA()
    {
        return $this->container['a'];
    }

    /**
     * Sets a
     *
     * @param \Svix\Svix\Models\EndpointDisabledEvent $a a
     *
     * @return self
     */
    public function setA($a)
    {
        $this->container['a'] = $a;

        return $this;
    }

    /**
     * Gets b
     *
     * @return \Svix\Svix\Models\EndpointCreatedEvent
     */
    public function getB()
    {
        return $this->container['b'];
    }

    /**
     * Sets b
     *
     * @param \Svix\Svix\Models\EndpointCreatedEvent $b b
     *
     * @return self
     */
    public function setB($b)
    {
        $this->container['b'] = $b;

        return $this;
    }

    /**
     * Gets c
     *
     * @return \Svix\Svix\Models\EndpointdUpdatedEvent
     */
    public function getC()
    {
        return $this->container['c'];
    }

    /**
     * Sets c
     *
     * @param \Svix\Svix\Models\EndpointdUpdatedEvent $c c
     *
     * @return self
     */
    public function setC($c)
    {
        $this->container['c'] = $c;

        return $this;
    }

    /**
     * Gets d
     *
     * @return \Svix\Svix\Models\EndpointDeletedEvent
     */
    public function getD()
    {
        return $this->container['d'];
    }

    /**
     * Sets d
     *
     * @param \Svix\Svix\Models\EndpointDeletedEvent $d d
     *
     * @return self
     */
    public function setD($d)
    {
        $this->container['d'] = $d;

        return $this;
    }

    /**
     * Gets e
     *
     * @return \Svix\Svix\Models\MessageAttemptExhaustedEvent
     */
    public function getE()
    {
        return $this->container['e'];
    }

    /**
     * Sets e
     *
     * @param \Svix\Svix\Models\MessageAttemptExhaustedEvent $e e
     *
     * @return self
     */
    public function setE($e)
    {
        $this->container['e'] = $e;

        return $this;
    }
    /**
     * Returns true if offset exists. False otherwise.
     *
     * @param integer $offset Offset
     *
     * @return boolean
     */
    public function offsetExists($offset)
    {
        return isset($this->container[$offset]);
    }

    /**
     * Gets offset.
     *
     * @param integer $offset Offset
     *
     * @return mixed|null
     */
    public function offsetGet($offset)
    {
        return $this->container[$offset] ?? null;
    }

    /**
     * Sets value based on offset.
     *
     * @param int|null $offset Offset
     * @param mixed    $value  Value to be set
     *
     * @return void
     */
    public function offsetSet($offset, $value)
    {
        if (is_null($offset)) {
            $this->container[] = $value;
        } else {
            $this->container[$offset] = $value;
        }
    }

    /**
     * Unsets offset.
     *
     * @param integer $offset Offset
     *
     * @return void
     */
    public function offsetUnset($offset)
    {
        unset($this->container[$offset]);
    }

    /**
     * Serializes the object to a value that can be serialized natively by json_encode().
     * @link https://www.php.net/manual/en/jsonserializable.jsonserialize.php
     *
     * @return mixed Returns data which can be serialized by json_encode(), which is a value
     * of any type other than a resource.
     */
    public function jsonSerialize()
    {
       return ObjectSerializer::sanitizeForSerialization($this);
    }

    /**
     * Gets the string presentation of the object
     *
     * @return string
     */
    public function __toString()
    {
        return json_encode(
            ObjectSerializer::sanitizeForSerialization($this),
            JSON_PRETTY_PRINT
        );
    }

    /**
     * Gets a header-safe presentation of the object
     *
     * @return string
     */
    public function toHeaderValue()
    {
        return json_encode(ObjectSerializer::sanitizeForSerialization($this));
    }
}

