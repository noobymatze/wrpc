import kotlinx.serialization.json.*

@JvmInline
value class Errors(
        private val errors: MutableList<JsonElement>,
) {

    fun error(element: JsonElement) {
        errors.add(element)
    }

    fun expect(type: String): JsonElement = buildJsonObject {
        put("@type", "Expected")
        put("type", type)
    }

    fun at(index: Int, error: JsonElement): JsonElement = buildJsonObject {
        put("@type", "At")
        put("index", index)
        put("error", error)
    }

    fun field(name: String, error: JsonElement): JsonElement = buildJsonObject {
        put("@type", "Field")
        put("name", name)
        put("error", error)
    }

    fun notNull(): JsonElement = buildJsonObject { put("@type", "NotNull") }
}
