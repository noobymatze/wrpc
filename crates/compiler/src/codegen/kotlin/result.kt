
sealed interface Result<out E, out T> {

    @JvmInline
    value class Ok<out T>(val value: T): Result<Nothing, T>

    @JvmInline
    value class Err<out E>(val error: E): Result<E, Nothing>

    companion object {

        fun <E, T> Result<E, T>.encode(
            encodeOk: JsonObjectBuilder.() -> Unit,
            encodeErr: JsonObjectBuilder.() -> Unit
        ): JsonObject = when (this) {
            is Ok -> buildJsonObject {
                put("type_", "Ok")
                encodeOk()
            }
            is Err -> buildJsonObject {
                put("type_", "Err")
                encodeErr()
            }
        }

        inline fun <E, T> decode(
            element: JsonElement?,
            required: Boolean,
            error: (JsonElement) -> Unit,
            decodeOk: (JsonObject) -> T,
            decodeErr: (JsonObject) -> E,
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
                "Ok" -> Result.Ok(decodeOk(element))
                "Err" -> Result.Err(decodeErr(element))
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

