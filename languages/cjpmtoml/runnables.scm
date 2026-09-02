
(table
    .
    "["
    .
    (bare_key) @_tablename
    .
    "]"
    (pair
        (bare_key) @_key
        .
        "="
        .
        (string) @_value) @run
    (#eq? @_tablename "package")
    (#eq? @_key "output-type")
    (#any-of? @_value "\"executable\"" "'executable'")
    (#set! tag cjpm-run))
