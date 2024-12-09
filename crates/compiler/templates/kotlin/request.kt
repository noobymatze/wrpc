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
        {%- for property in record.get_validation_ordered_properties() %}
        {% if !property.constraints.is_empty() %}
        {%- let var_property_valid = format!("{}Valid", property.name.value) %}
        var {{ var_property_valid }} = false
        val {{var_property_valid}}Deps = {{ self::deps_condition(property) }}
        if ({{var_property_valid}}Deps) {
            {%- for (i, constraint) in property.constraints.iter().enumerate() %}
            {{var_property_valid}} = {{ self::condition("this", constraint) }}
            if (!{{var_property_valid}}) {
                errors.error("")
            }
            {% endfor %}
        }

        {%- endif -%}
        {%- endfor -%}

        {%- for constraint in record.constraints -%}
        if (!({{ self::condition("this", constraint) }})) {
            errors.error("")
        }
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
