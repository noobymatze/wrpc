{% if !record.is_simple() %}
{{ self::generate_doc_comment("", record.comment) }}
export type {{ record.name.value }} =
    {%- for variant in record.variants %}
    | { 
        '@type': "{{ variant.name.value}}";
    {%- for property in variant.properties %}
        {{property.name.value}}: {{ self::generate_type_ref(package, property.type_) }};
    {%- endfor %}
      }
    {%- endfor %};
{% else %}
{{ self::generate_doc_comment("", record.comment) }}
export type {{ record.name.value }} =
    {%- for variant in record.variants %}
    | "{{ variant.name.value}}"
    {%- endfor %};
{% endif %}
