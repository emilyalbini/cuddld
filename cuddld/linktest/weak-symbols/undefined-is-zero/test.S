.intel_syntax noprefix
.file "test.S"

.weak undefined
.global _start

.section .text
_start:
    mov eax, undefined
