{% if !imports.is_empty() %}
import { {{imports}} } from './models.ts';
{% endif %}

export interface Client {
    {%- for service in services %}
    {{ service.name.uncapitalized() }}: {{ service.name.value }};
    {%- endfor %}
}

{% for service in services %}
export interface {{ service.name.value }} {
    {%- for method in service.get_sorted_methods() %}
{{ self::generate_doc_comment("    ", method.comment) }}
    {{ method.name.uncapitalized() }}: (
    {%- for param in method.parameters %}
        {{ param.name.value }}: {{ self::generate_type_ref(package, param.type_) }}{% if !loop.last %},{% endif %}
    {%- endfor %}
    ){% if let Some(return_type) = method.return_type %} => Promise<HttpResponse<{{ self::generate_type_ref(package, return_type) }}>>{% else %} => Promise<HttpResponse<void>>{% endif %}
    {%- endfor %}
}
{% endfor %}

{% for service in services %}
export function {{ service.name.value }}(baseUrl: string): {{ service.name.value }} {
    return {
        {%- for method in service.get_sorted_methods() %}
        {{ method.name.value }}(
        {%- for param in method.parameters %}
            {{ param.name.value }}: {{ self::generate_type_ref(package, param.type_) }}{% if !loop.last %},{% endif %}
        {%- endfor %}
        ){% if let Some(return_type) = method.return_type %}: Promise<HttpResponse<{{ self::generate_type_ref(package, return_type) }}>>{% else %}: Promise<HttpResponse<void>>{% endif %} {
            return request(baseUrl, "{{ service.get_method_path(method) }}", {
                {%- for param in method.parameters %}
                {{ param.name.value }}{% if !loop.last %},{% endif %}
                {%- endfor %}
            });
        }{%if !loop.last %},{% endif %}
        {% endfor %}
    }
}
{% endfor %}

/**
 * Represents an http response.
 */
export type HttpResponse<T>
    = { '@type': 'Ok'; value: T; }
    | { '@type': 'Err'; error: HttpError; }

/**
 * Represents any error, that could happen during a request.
 */
export type HttpError
    = { type: 'Network', }
    | { type: 'Timeout', }
    | { type: 'BadUrl', }
    | { type: 'BadStatus', statusCode: number, headers: Headers, body: string }
    | { type: 'BadBody', };

/**
 * Returns a function, that can be used to call the given method
 * for an rpc.
 *
 * @param baseUrl
 * @param path
 */
async function request<Params, Ret>(
    baseUrl: string,
    path: string,
    params: Params,
): Promise<HttpResponse<Ret>> {
    try {
        const response = await fetch(`${baseUrl}${path}`, {
            method: "POST",
            body: JSON.stringify(params),
            headers: {
                "Content-Type": "application/json",
            },
        });

        try {
            if (!response.ok) {
                const statusCode = response.status;
                const body = await response.text();
                const headers = response.headers;
                return {'@type': 'Err', error: {type: 'BadStatus', statusCode, headers, body}};
            }

            const value = await response.json();
            return {'@type': 'Ok', value };
        } catch (error) {
            return {'@type': 'Err', error: {type: 'BadBody'}};
        }
    } catch (error) {
        if (error instanceof DOMException && error.message === 'Timeout') {
            return {'@type': 'Err', error: {type: 'Timeout'}};
        }

        return {'@type': 'Err', error: {type: 'Network'}};
    }
}