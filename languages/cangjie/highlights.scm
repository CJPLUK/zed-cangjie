; base item
(varBindingPattern) @variable
(scoped_identifier
    scope: (identifier) @variable)
(atomicVariable
    .
    (identifier) @variable)

(className) @type.class
(structName) @type.class
(enumName) @type.class
(extendType) @type.class
(classBody
    (init
        "init" @constructor))
(structBody
    (init
        "init" @constructor))
(classBody
    (primaryInit
        .
        (className) @constructor))
(structBody
    (primaryInit
        .
        (structName) @constructor))
(superOrInterface) @type
(interfaceName) @type.interface
(packageDeclaration
    "package" @keyword.package
    packageName: (_) @label)
(importList
    "import" @keyword
    )
(packageFull
    packageName: (_) @label
    (asterisk)? @punctuation.special)
(scoped_identifier
    "." @punctuation.delimiter
    name: _ @label)

; keywords
; (variableDeclaration
;     "let" @keyword)
; (variableDeclaration
;     "var" @keyword)
; (variableDeclaration
;     "const" @keyword)
; (unnamedMemberParam
;     "let" @keyword)
; (unnamedMemberParam
;     "var" @keyword)
; (jumpExpression
;     "throw" @keyword)
; (jumpExpression
;     "return" @keyword)
; (jumpExpression
;     "continue" @keyword)
; (jumpExpression
;     "break" @keyword)
(modifiers) @keyword
[
    "let"
    "var"
    "const"
    "throw"
    "return"
    "continue"
    "break"
    "unsafe"
] @keyword

(macroExpression
    "@" @preproc
    (macroName) @preproc)

(foreignDeclaration
    "foreign" @keyword)
(classDefinition
    "class" @keyword)
(structDefinition
    "struct" @keyword)
(interfaceDefinition
    "interface" @keyword)
(extendDefinition
    "extend" @keyword)
(enumDefinition
    "enum" @keyword)
(enumBody
    enumConstant: (identifier) @constant)
(functionDefinition
    "func" @keyword
    (funcName) @function
    "where"? @keyword)
(mainDefinition
    "main" @function.main)
(parameter
    paraName: (_) @variable.parameter
    type: (_) @type)
(namedParameter
    paraName: (_) @variable.parameter
    type: (_) @type)
(callSuffix
    (varBindingPattern) @variable.parameter)
(typeParameters
    (identifier) @type)
(returnType
    ":" @punctiation.delimiter
    _ @type)
(genericConstraint
    (identifier) @type)
(userType
    (identifier) @type)
(fieldAccess
    "."
    (_) @property)
(thisSuperExpression) @variable.special

; Control flow expressions
(tryExpression
    "try" @keyword)
(tryExpression
    "catch" @keyword)
(tryExpression
    "finally" @keyword)
(catchPattern
    ":" @punctiation.delimiter
    _ @type)
(ifExpression
    "if" @keyword)
(ifExpression
    alternative: "else" @keyword)
(forInExpression
    "for" @keyword)
(forInExpression
    "in" @keyword)
(forInExpression
    "(" @punctuation.bracket
    ")" @punctuation.bracket)
(whileExpression
    "while" @keyword)
(matchExpression "match" @keyword)
(matchCase "case" @keyword)
(matchCaseBody "case" @keyword)


; member access
(scoped_identifier
    name: (identifier) @attribute)
(atomicVariable
    (fieldAccess
        (identifier) @attribute)
    . )
; constant
((varBindingPattern) @constant
    (#match? @constant "^[A-Z0-9_]*$"))
((identifier) @constant
    (#match? @constant "^[A-Z0-9_]*$"))
(scoped_identifier
    name: (identifier) @constant
    (#match? @constant "^[A-Z0-9_]*$"))
; (atomicVariable
;     .
;     (identifier) @keyword
;     (#match? @keyword "^[A-Z0-9_]*$"))
; attributes
(propertyDefinition
    "prop" @keyword
    (propertyName) @attribute
    getter: "get"? @function
    setter: "set"? @function)
(classBody
    (variableDeclaration
        (variableName
            (varBindingPattern) @attribute)))
(primaryInitParamList
    (unnamedMemberParam
        (parameter
            paraName: (identifier) @attribute)))
(structBody
    (variableDeclaration
        (variableName
            (varBindingPattern) @attribute)))

; plain function call
(postfixExpression
    (atomicVariable
        . (identifier) .) @function
    (callSuffix))
; method call
(postfixExpression
    (fieldAccess
        "."
        (_) @function)
    .
    (callSuffix))
; a.b(...)
(postfixExpression
    (atomicVariable
        (scoped_identifier
            name: (identifier) @function)
        . )
    .
    (callSuffix))
; a[0].b(...)
(postfixExpression
    (atomicVariable
        (fieldAccess
            (identifier) @function)
        . )
    .
    (callSuffix))
; super.b(...)
(
    (fieldAccess
        (identifier) @function)
    .
    (callSuffix))


(callSuffix
    "(" @punctiation.bracket
    ")" @punctiation.bracket)
(parenthesizedExpression
    "(" @punctiation.bracket
    ")" @punctiation.bracket)
(fieldAccess
    "." @punctuation.delimiter)
(typeArguments
    "<" @punctuation.bracket
    ">" @punctuation.bracket)
(indexAccess
    "[" @operator
    "]" @operator)

; literals
(stringLiteral) @string
(inlineExpression
    "${" @punctuation.bracket
    "}" @punctuation.bracket
    ) @embedded
(integerLiteral) @number
(floatLiteral) @number
((identifier) @boolean
    (#any-of? @boolean "true" "false"))

[
    "where"
] @keyword
[
    "++"
    "."
] @operator
[
    "<-"
    "="
] @operator.special
[
    "<:"
    "=>"
    ","
] @punctuation.delimiter
(binaryExpreesion
    operator: _ @operator)
(assignmentExpression
    operator: _ @operator)
; [
;     "Int64"
; ] @type
; (Int64) @type ; There seems to be a node as well as a token
; (booleanLiteral) @constant
; (varBindingPattern) @variable
; [
;     (integerLiteral)
;     (floatLiteral)
; ] @number
; (modifiers) @keyword
; (className) @type
; (structName) @type
; (interfaceName) @type
; (Bool) @type
; (typeParameters
;     (identifier) @type)
; (superOrInterface) @type
;
; (funcName) @function
; (propertyName) @property
;
;
[
    (blockComment)
    (lineComment)
] @comment
;
[
    "Int8"
    "Int16"
    "Int32"
    "Int64"
    "IntNative"
    "UInt8"
    "UInt16"
    "UInt32"
    "UInt64"
    "UIntNative"
    "Float16"
    "Float32"
    "Float64"
    "Rune"
    "Bool"
    "Unit"
    "Nothing"
    ; "This"
    (Int8)
    (Int16)
    (Int32)
    (Int64)
    (IntNative)
    (UInt8)
    (UInt16)
    (UInt32)
    (UInt64)
    (UIntNative)
    (Float16)
    (Float32)
    (Float64)
    (Rune)
    (Bool)
    (Unit)
    (Nothing)
] @type.builtin
(atomicVariable
    .
    (identifier) @type.builtin
    (#eq? @type.builtin "Option"))
