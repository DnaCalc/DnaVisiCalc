#include <stddef.h>
#include <string.h>

/* MinGW/Windows toolchains used by this repo may miss memmem. */
void *memmem(const void *haystack, size_t haystacklen, const void *needle, size_t needlelen) {
  const unsigned char *h = (const unsigned char *)haystack;
  const unsigned char *n = (const unsigned char *)needle;
  size_t i;

  if (needlelen == 0) {
    return (void *)haystack;
  }
  if (!haystack || !needle || haystacklen < needlelen) {
    return NULL;
  }

  for (i = 0; i + needlelen <= haystacklen; i++) {
    if (h[i] == n[0] && memcmp(h + i, n, needlelen) == 0) {
      return (void *)(h + i);
    }
  }
  return NULL;
}
