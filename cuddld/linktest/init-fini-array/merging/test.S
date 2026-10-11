.intel_syntax noprefix
.file "foo.S"

.global _start

.section .text
_start:
    nop
init_fn_1:
    nop
init_fn_2:
    nop
init_fn_3:
    nop
fini_fn_1:
    nop
fini_fn_2:
    nop
fini_fn_3:
    nop

.section .init_array.1,"aw",@init_array
    .quad init_fn_1

.section .init_array.2,"aw",@init_array
    .quad init_fn_2

.section .init_array,"aw",@init_array
    .quad init_fn_3

.section .fini_array.1,"aw",@fini_array
    .quad fini_fn_1

.section .fini_array.2,"aw",@fini_array
    .quad fini_fn_2

.section .fini_array,"aw",@fini_array
    .quad fini_fn_3
