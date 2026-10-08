/* Linux-only benchmark fault: retain 64 MiB and delay stdout writes by 10 ms.
 * Build with cc -shared -fPIC -O2 -Wall -Wextra -Werror, then preload only into
 * an owned LSP process. Never use this library in production or normal CI.
 */
#define _GNU_SOURCE
#include <stddef.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <sys/uio.h>
#include <time.h>
#include <unistd.h>

__attribute__((constructor)) static void retain_memory(void) {
  const size_t bytes = 64 * 1024 * 1024;
  volatile unsigned char *memory =
      mmap(NULL, bytes, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  if (memory == MAP_FAILED)
    _exit(120);
  for (size_t index = 0; index < bytes; index += 4096)
    memory[index] = 1;
}

static void delay_stdout(int fd) {
  if (fd == STDOUT_FILENO) {
    const struct timespec delay = {0, 10000000};
    nanosleep(&delay, NULL);
  }
}

ssize_t write(int fd, const void *buffer, size_t count) {
  delay_stdout(fd);
  return syscall(SYS_write, fd, buffer, count);
}

ssize_t writev(int fd, const struct iovec *vectors, int count) {
  delay_stdout(fd);
  return syscall(SYS_writev, fd, vectors, count);
}
