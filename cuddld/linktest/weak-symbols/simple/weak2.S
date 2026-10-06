.intel_syntax noprefix
.file "weak2.S"

.weak hello

.section .text
hello:
    nop
