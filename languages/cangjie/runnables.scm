(
    (packageDeclaration
        "package"
        .
        packageName: (_) @pkgname)
    (macroExpression
        "@"
        .
        (macroName) @_macroname
        (#eq? @_macroname "Test"))
    .
    (functionDefinition
        "func"
        .
        (funcName) @testname @run)
    (#set! tag cjpm-test)
)

(mainDefinition . "main" @run
    (#set! tag cjpm-run))
