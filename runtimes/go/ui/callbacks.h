/*
 * The C side of package ui: the tables of callbacks the library calls,
 * each a small function that hands the call to the Go function exported
 * for it (callbacks.c). A Go value crosses as a cgo.Handle, made a pointer
 * here so Go never turns a number into a pointer.
 */

#ifndef CLARITY_GO_CALLBACKS_H
#define CLARITY_GO_CALLBACKS_H

#include <stdint.h>
#include <clarity_ui.h>

/* A ClarityProgram whose callbacks answer through the Go program behind `handle`. */
ClarityStatus clarity_go_program_new(uintptr_t handle, ClarityProgram **out);

/* Registers custom part `kind`, made by the Go maker behind `handle`. */
ClarityStatus clarity_go_custom_register(const char *kind, size_t kind_len, uintptr_t handle);

/* The thread that calls, as a number: a long call's watch belongs to the thread its handler runs on. */
uint64_t clarity_go_thread(void);

#endif
