#include <stdint.h>

const uint16_t S_IFMT  = 0170000;
const uint16_t S_IFDIR = 0040000;
const uint16_t S_IFREG = 0100000;
const uint16_t S_IFLNK = 0120000;

struct stat {
  uint64_t size;
  uint16_t mode;
  int64_t  atime;
  int64_t  mtime;
  int64_t  ctime;
};

struct mountbox_operations {
  int (*open)(const char * path);
  int (*close)(const char * path, uint64_t fh);
  int (*read)(const char * path, char * buf, uint64_t size, int64_t offset, uint64_t fh);
  int (*getattr)(const char * path, struct stat * stat);
};

static struct mountbox_operations operations;