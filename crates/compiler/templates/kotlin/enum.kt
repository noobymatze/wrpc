
package {{ package }}.models

import kotlinx.serialization.json.*

{% if !record.is_simple() %}
sealed class {{record.name.value}} {

    {%- for variant in record.variants %}
    {%- if variant.properties.is_empty() %}
    data object {{ variant.name.value }}: {{ record.name.value }}
    {% else %}
    data class {{ variant.name.value }}(
    {%- for property in variant.properties %}
        val {{property.name.value}}: {{ self::generate_type_ref(package, property.type_) }},
    {%- endfor %}
    ): {{ record.name.value }}
    {% endif %}
    {%- endfor %}

    fun encode(): JsonElement = 
        when (this) {
            {%- for variant in record.variants %}
            is {{ variant.name.value }} -> buildJsonObject {
                put("@type", "{{ variant.name.value }}")
                {%- for property in variant.properties %}
                put("{{ property.name.value }}", {{ self::encode_type(property.name.value.as_str(), property.type_) }})
                {%- endfor %}
            }
            {%- endfor %}
        }

    companion object {

        fun decode(json: JsonElement, errors: ErrorBundle): {{record.name.value}}? {
            if (json !is JsonObject) {
                errors.error(errors.expect("OBJECT"))
                return null
            }

            val typeField_ = json["@type"]
            val type_ = if (typeField_ !is JsonPrimitive || !typeField_.isString) {
                errors.error(errors.field("@type", errors.expect("STRING")))
                return null
            } else {
                typeField_.content
            }

            when (type_) {
                {%- for variant in record.variants %}
                "{{ variant.name.value }}" -> {
                    {%- for property in variant.properties %}
{{ self::decode_property("                    ", "json", "errors", property) }}
                    {% endfor %}

                    if (errors.isEmpty()) {
                        val {{ variant.name.uncapitalized() }} = {{ variant.name.value }}(
                            {%- for property in variant.properties %}
                            {%- let required = !matches!(property.type_, Type::Option(_)) %}
                            {{ property.name.value }} = {{ property.name.value }}{% if required %}!!{% endif %},
                            {%- endfor %}
                        )
                        return {{ variant.name.uncapitalized() }}
                    } else {
                        return null
                    }
                }
                {%- endfor %}
                else -> {
                    errors.error(errors.field("@type", errors.expect("UNKNOWN")))
                    return null
                }
            }
        }

    }

}
{% else %}
enum class {{ record.name.value }} {
    {%- for variant in record.variants %}
    {{ variant.name.value }}{% if !loop.last %},{% else %};{% endif %}
    {%- endfor %}

    fun encode(): JsonElement = 
        JsonPrimitive(toString())

    companion object {

        fun decode(json: JsonElement, errors: ErrorBundle): {{ record.name.value }} {
            if (json !is JsonPrimitive || !json.isString) {
                errors.error(errors.expect("STRING"))
                return null
            }

            val value = json.content
            return try {
                valueOf(value)
            } catch (ex: IllegalArgumentException) {
                errors.error(errors.expect("ONEOF"))
                null
            }
        }

    }

}
{% endif %}
