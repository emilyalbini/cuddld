.intel_syntax noprefix
.file "features1_both.S"

.global _start

.section .text
_start:
    nop

#include "shared.S"
GNU_PROPERTY_UINT32 GNU_PROPERTY_X86_FEATURES_1_AND 0b11
