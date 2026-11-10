#include <errno.h>
#include <libproc.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>

int main(int argc, char **argv) {
  if (argc < 2)
    return 2;
  printf("[");
  int printed = 0;
  for (int i = 1; i < argc; i++) {
    char *end;
    long pid = strtol(argv[i], &end, 10);
    if (*end || pid <= 0)
      return 2;
    struct rusage_info_v4 info = {0};
    if (proc_pid_rusage((int)pid, RUSAGE_INFO_V4, (rusage_info_t *)&info) !=
        0) {
      if (errno == ESRCH)
        continue;
      return 1;
    }
    printf("%s{\"pid\":%ld,\"rss\":%llu,\"footprint\":%llu,\"cpu_ns\":%llu,"
           "\"lifetime_peak_footprint\":%llu}",
           printed++ == 0 ? "" : ",", pid, info.ri_resident_size,
           info.ri_phys_footprint, info.ri_user_time + info.ri_system_time,
           info.ri_lifetime_max_phys_footprint);
  }
  printf("]\n");
  return printed ? 0 : 1;
}
