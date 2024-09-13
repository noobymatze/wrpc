package {{ package }}.services

import {{ package }}.models
import kotlinx.serialization.json.*

interface {{ service.name.value }} {

    {%- for property in record.properties %}
    {% let field_name = format!("{}Field", property.name.value) %}
    val {{ field_name }} = json["{{ property.name.value }}"]
    val {{property.name.value}}: {{ self::generate_type_ref(package, property.type_) }}? = null
    if ({{ field_name }} !)
    {%- endfor %}
    




    companion object {

        fun decode(json: JsonElement, errors: ErrorBundle): {{record.name.value}}? {
            if (json !is JsonObject) {
                errors.error(errors.expect("OBJECT"))
                return null
            }

            {%- for property in record.properties %}
            {% let field_name = format!("{}Field", property.name.value) %}
            val {{ field_name }} = json["{{ property.name.value }}"]
            val {{property.name.value}}: {{ self::generate_type_ref(package, property.type_) }}? = null
            if ({{ field_name }} !)
            {%- endfor %}
        }

    }

}