{{ self::generate_doc_comment("", record.comment) }}
export type {{ record.name.value }} = {
    {%- for property in record.properties %}
    {{ property.name.value }}: {{ self::generate_type_ref(package, property.type_) }};
    {%- endfor %}
}