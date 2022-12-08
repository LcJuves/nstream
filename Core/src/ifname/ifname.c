#include <net/if_utun.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/sys_domain.h>

#include "ifname.h"

int utun_ifname(char *utunname, int fd, size_t size) {
    // Get iface name of newly created utun dev.
    socklen_t utunname_len = (socklen_t)sizeof(utunname);
    return getsockopt(fd, SYSPROTO_CONTROL, UTUN_OPT_IFNAME, (void *)utunname,
                      &utunname_len);
}