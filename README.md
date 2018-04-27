# LLVM 4.0 to 6.0 loop unroll change

When Rust changed from LLVM 4.0 to LLVM 6.0, Firefox's UTF-8 validation
function regressed in performance on i686 and also (but less) on x86_64 when
used on ASCII input.

This repo contains the minimized program that extracts the innermost loop
whose generated code changed.

## Scripts

* `asm.sh` removes the target directory, compiles the program into asm and
  opens `less` viewing the asm.
* `test.sh` runs the correctness test.
* `bench.sh` runs the benchmark.

## The difference

```sh
rustup default 1.24.0
./asm.sh
```

shows one instance of `movdqu` instruction. The basic block for the loop body
is as straight-forward as one would expect:

```asm
.LBB0_6:
        movdqu  (%rdi,%rax), %xmm0
        pmovmskb        %xmm0, %edx
        testl   %edx, %edx
        jne     .LBB0_7
        addq    $16, %rax
        cmpq    %rcx, %rax
        jbe     .LBB0_6
        jmp     .LBB0_2
```

```sh
rustup default 1.25.0
./asm.sh
```

shows two instances of the `movdqu` instruction. It looks like the first trip
through the loop has been unrolled (the unrolling shows up in LLVM IR, too)
and this causes the contents of the basic block for the actual loop to rotate
within the basic block:

```asm
        .cfi_startproc
        cmpq    $16, %rsi
        jb      .LBB0_1
        movdqu  (%rdi), %xmm0
        pmovmskb        %xmm0, %ecx
        testl   %ecx, %ecx
        je      .LBB0_10
        xorl    %esi, %esi
        testl   %ecx, %ecx
        je      .LBB0_7
.LBB0_8:
        bsfl    %ecx, %eax
        jmp     .LBB0_9
.LBB0_1:
        xorl    %eax, %eax
        cmpq    %rsi, %rax
        jb      .LBB0_13
.LBB0_15:
        movq    %rsi, %rax
        retq
.LBB0_10:
        leaq    -16(%rsi), %rdx
        movl    $16, %eax
        .p2align        4, 0x90
.LBB0_11:
        cmpq    %rdx, %rax
        ja      .LBB0_12
        movdqu  (%rdi,%rax), %xmm0
        pmovmskb        %xmm0, %ecx
        addq    $16, %rax
        testl   %ecx, %ecx
        je      .LBB0_11
        addq    $-16, %rax
        movq    %rax, %rsi
        testl   %ecx, %ecx
        jne     .LBB0_8
```

Both inner loops execute the same number of instructions in the case where
the looping continues. Yet, on the EC2 instances that Mozilla uses for
Firefox CI the new form caused an up to 12.5% performance regression. I've
been told the instance types are c4.2xlarge and c3.xlarge (Haswell and Ivy
Bridge).

## Performance results

### x86_64 code running on Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz (Broadwell-EP)

Similar results with both the `powersave` and `performance` governors.

```
$ rustup default 1.24.0
$ ./bench.sh
[...]
test bench ... bench:   1,539,341 ns/iter (+/- 216,985)
```

```
$ rustup default 1.25.0
$ ./bench.sh
[...]
test bench ... bench:   1,865,801 ns/iter (+/- 22,297)
```
