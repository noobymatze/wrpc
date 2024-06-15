
import kotlinx.serialization.json.*

fun at(index: Int, error: JsonElement): JsonObject = buildJsonObject {
    put("type_", "At")
    put("index", index)
    put("error", error)
}

fun expected(kind: String, found: JsonElement?): JsonObject = buildJsonObject {
    put("type_", "Expected")
    put("type", kind)
    put("found", found ?: JsonNull)
}

fun required(): JsonObject = buildJsonObject {
    put("type_", "Required")
}

inline fun <T> decodePrimitive(
    element: JsonElement?,
    required: Boolean,
    kind: String,
    error: (JsonElement) -> Unit,
    extract: (JsonPrimitive) -> T?,
): T? {
    if (element == null || element == JsonNull) {
        if (required) error(required())
        return null
    }

    if (element !is JsonPrimitive) {
        error(expected(kind, element))
        return null
    }

    val value = extract(element)
    if (value == null) {
        error(expected(kind, element))
        return null
    }

    return value
}

inline fun <T> decodeArray(
    element: JsonElement?,
    required: Boolean,
    kind: String,
    decode: (JsonElement, (JsonElement) -> Unit) -> T,
    error: (JsonElement) -> Unit,
): List<T>? {
    if (element == null || element == JsonNull) {
        if (required) error(required())
        return null
    }

    if (element !is JsonArray) {
        error(expected(kind, element))
        return null
    }

    val errors = mutableListOf<JsonElement>()
    val values = mutableListOf<T>()
    for ((i, value) in element.withIndex()) {
        values.add(decode(value) { errors.add(at(i, it)) })
    }

    if (errors.isNotEmpty()) {
        error(JsonArray(errors))
        return null
    }

    return values
}

inline fun <T> decodeObject(
    element: JsonElement?,
    required: Boolean,
    error: (JsonElement) -> Unit,
    decode: JsonObject.() -> T?,
): T? {
    if (element == null || element == JsonNull) {
        if (required) error(required())
        return null
    }

    if (element !is JsonObject) {
        error(expected("OBJECT", element))
        return null
    }

    return decode(element)
}

inline fun decodeString(json: JsonElement?, required: Boolean, error: (JsonElement) -> Unit): String? =
    decodePrimitive(json, required, "STRING", error) { it.contentOrNull }

inline fun decodeInt32(json: JsonElement?, required: Boolean, error: (JsonElement) -> Unit): Int? =
    decodePrimitive(json, required, "INT32", error) { it.intOrNull }

inline fun decodeInt64(json: JsonElement?, required: Boolean, error: (JsonElement) -> Unit): Long? =
    decodePrimitive(json, required, "INT64", error) { it.longOrNull }

inline fun decodeFloat32(json: JsonElement?, required: Boolean, error: (JsonElement) -> Unit): Float? =
    decodePrimitive(json, required, "FLOAT32", error) { it.floatOrNull }

inline fun decodeFloat64(json: JsonElement?, required: Boolean, error: (JsonElement) -> Unit): Double? =
    decodePrimitive(json, required, "FLOAT64", error) { it.doubleOrNull }

inline fun decodeBoolean(json: JsonElement?, required: Boolean, error: (JsonElement) -> Unit): Boolean? =
    decodePrimitive(json, required, "BOOLEAN", error) { it.booleanOrNull }
