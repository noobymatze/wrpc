
sealed interface Result<out E, out T> {

    @JvmInline
    value class Ok<out T>(val value: T): Result<Nothing, T>

    @JvmInline
    value class Err<out E>(val error: E): Result<E, Nothing>

    fun encode(
        encodeOk: (T) -> JsonElement?,
        encodeErr: (E) -> JsonElement?,
    ): JsonObject = when (this) {
        is Ok -> buildJsonObject {
            put("type_", "Ok")
            put("value", encodeOk(value) ?: JsonNull)
        }
        is Err -> buildJsonObject {
            put("type_", "Err")
            put("error", encodeErr(error) ?: JsonNull)
        }
    }


    companion object {

        inline fun <E, T> decode(
            element: JsonElement?,
            required: Boolean,
            error: (JsonElement) -> Unit,
            decodeOk: (JsonElement?) -> T,
            decodeErr: (JsonElement?) -> E,
        ): Result<E, T>? {
            if (element == null || element == JsonNull) {
                if (required) error(required())
                return null
            }

            if (element !is JsonObject) {
                if (required) error(expected("OBJECT", element))
                return null
            }

            val discriminator = element["type_"]
            if (discriminator !is JsonPrimitive || !discriminator.isString) {
                error(buildJsonObject {
                    put("type_", "BadDiscriminator")
                    put("error", expected("STRING", discriminator))
                })
                return null
            }

            return when (discriminator.content) {
                "Ok" -> Ok(decodeOk(element["value"]))
                "Err" -> Err(decodeErr(element["error"]))
                else -> {
                    error(buildJsonObject {
                        put("type_", "BadDiscriminator")
                        put("error", expected("STRING", discriminator))
                    })
                    null
                }
            }
        }

    }

}

