.intel_syntax noprefix
.file "foo.S"

.global _start

.section .text
_start:
    nop
init_fn:
    nop
fini_fn:
    nop

.section .init_array,"aw",@init_array
    .quad init_fn

.section .fini_array,"aw",@fini_array
    .quad fini_fn
