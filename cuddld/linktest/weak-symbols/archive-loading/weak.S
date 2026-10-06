.intel_syntax noprefix
.file "weak.S"

.weak hello

.section .text
hello:
    nop
