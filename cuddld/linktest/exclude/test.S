.intel_syntax noprefix
.file "test.S"

.global _start

.section .data
    msg:
    .ascii "Hello world!\n"

.section .text
_start:
    nop

.section .text.excluded, "e"
    nop
