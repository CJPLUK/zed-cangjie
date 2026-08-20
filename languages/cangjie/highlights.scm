; (identifier) @variable ; Make this get easily overriden
(className) @type.class
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
(variableDeclaration
    "let" @keyword)
(variableDeclaration
    "var" @keyword)
(unnamedMemberParam
    "let" @keyword)
(unnamedMemberParam
    "var" @keyword)
(jumpExpression
    "throw" @keyword)
(jumpExpression
    "return" @keyword)
(jumpExpression
    "continue" @keyword)
(jumpExpression
    "break" @keyword)
(modifiers) @keyword

(classDefinition
    "class" @keyword)
(interfaceDefinition
    "interface" @keyword)
(extendDefinition
    "extend" @keyword)
(functionDefinition
    "func" @keyword
    (funcName) @function
    "where"? @keyword)
(mainDefinition
    "main" @function.main)
(parameter
    paraName: (_) @variable.parameter
    type: (_) @type)
(typeParameters
    (identifier) @type)
(returnType
    ":" @punctiation.delimiter
    _ @type)
(fieldAccess
    "."
    (_) @property)
(thisSuperExpression) @variable.special
(tryExpression
    "try" @keyword
    "catch"? @keyword
    ; "handle"? @keyword
    "finally"? @keyword)
(catchPattern
    ":" @punctiation.delimiter
    _ @type)

;
(varBindingPattern) @variable
(stringLiteral) @string
; plain function call
(postfixExpression
    (atomicVariable) @function
    (callSuffix))
; method call
(postfixExpression
    (fieldAccess
        "."
        (_) @function)
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

(integerLiteral) @number

[
    "where"
] @keyword
[
    "<:"
] @punctuation.delimiter
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
; [
;     "Int8"
;     "Int16"
;     "Int32"
;     "Int64"
;     "IntNative"
;     "UInt8"
;     "UInt16"
;     "UInt32"
;     "UInt64"
;     "UIntNative"
;     "Float16"
;     "Float32"
;     "Float64"
;     "Rune"
;     "Bool"
;     "Unit"
;     "Nothing"
;     ; "This"
; ] @type
