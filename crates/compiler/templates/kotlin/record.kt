package {{ package }}.models

import kotlinx.serialization.json.*

{% if !record.properties.is_empty() %}
data class {{record.name.value}}(
{%- for property in record.properties %}
    val {{property.name.value}}: {{ self::generate_type_ref(package, property.type_) }},
{%- endfor %}
) {

    fun encode(): JsonElement = buildJsonObject {
        {%- for property in record.properties %}
        put("{{ property.name.value }}", {{ property.name.value }})
        {%- endfor %}
    }

    fun validate(): ErrorBundle {
        val errors = ErrorBundle()
        {%- for annotation in record.annotations %}
        {%- for constraint in annotation.get_constraints() %}
        if (!({{ self::condition("this", constraint) }})) {
            errors.error("")
        }
        {% endfor %}
        {% endfor %}
        {%- for property in record.properties %}
        {%- for annotation in property.annotations %}
        {%- for constraint in annotation.get_constraints() %}
        if (!({{ self::condition("this", constraint) }})) {
            errors.error(errors.field("{{ property.name.value }}", errors.expect("STRING")))
        }
        {% endfor -%}
        {% endfor -%}
        {% endfor %}
        return errors
    }

    companion object {

        fun decode(json: JsonElement, errors: ErrorBundle): {{record.name.value}}? {
            if (json !is JsonObject) {
                errors.error(errors.expect("OBJECT"))
                return null
            }

            {% for property in record.properties %}
{{ self::decode_property("            ", "json", "errors", property) }}
            {% endfor %}

            if (errors.isEmpty()) {
                val {{ record.name.uncapitalized() }} = {{ record.name.value }}(
                    {%- for property in record.properties %}
                    {%- let required = !matches!(property.type_, Type::Option(_)) %}
                    {{ property.name.value }} = {{ property.name.value }}{% if required %}!!{% endif %},
                    {%- endfor %}
                )
                return {{ record.name.uncapitalized() }}
            } else {
                return null
            }
        }

    }

}
{% else %}
data object {{ record.name.value }} {

    companion object {

        fun decode(json: JsonElement, errors: ErrorBundle): {{ record.name.value }} {
            if (json !is JsonObject) {
                errors.error(errors.expect("OBJECT"))
                return null
            }

            return {{ record.name.value }}
        }

    }

}
{% endif %}
