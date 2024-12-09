package {{ package }}.services

import io.ktor.http.*
import io.ktor.server.application.*
import io.ktor.server.request.*
import io.ktor.server.response.*
import io.ktor.server.routing.*
import kotlinx.serialization.json.*
import {{ package }}.json.*
import {{ package }}.models.*

{{ self::generate_doc_comment("", service.comment) }}
interface {{ service.name.value }} {
    {% for method in service.get_sorted_methods() %}
{{ self::generate_doc_comment("    ", method.comment) }}
    fun {{ method.name.value }}(
    {%- for parameter in method.parameters %}
        {{ parameter.name.value }}: {{ self::generate_type_ref(package, parameter.type_) }},
    {%- endfor %}
    )
    {% endfor %}

    companion object {

        /**
         * Mount all methods of the [{{ service.name.value }}].
         *
         * @param service the service to mount
         */
        fun Routing.service(service: {{ service.name.value }}) {
            {%- for method in service.get_sorted_methods() %}
            post("{{ service.get_method_path(method) }}") {
                try {
                    {%- if !method.parameters.is_empty() %}
                    val params = call.receiveNullable<JsonElement>()
                    val errors = ErrorBundle()

                    if (params !is JsonObject) {
                        errors.error(errors.expect("OBJECT"))
                        return@post
                    }
                    {% for param in method.parameters %}
{{ self::decode_parameter("                    ", "params", "errors", param) }}
                    {% endfor -%}
                    {% endif %}

                    {%- if let Some(return_type) = method.return_type %}
                    val result = service.{{ method.name.value }}()

                    {%- if matches!(return_type, Type::Option(_)) %}
                    call.respondNullable(result?.let { {{ self::encode_type("it", return_type) }} })
                    {% else %}
                    call.respond({{ self::encode_type("result", return_type) }})
                    {%- endif -%}

                    {% else %}
                    service.{{ method.name.value }}()
                    call.respond(HttpStatusCode.NoContent)
                    {%- endif %}
                } catch (t: Throwable) {
                    call.respond(HttpStatusCode.InternalServerError)
                }
            }
            {%- endfor %}
        }

    }

}
