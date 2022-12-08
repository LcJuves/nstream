#include <sys/socket.h>
#include <sys/sys_domain.h>
#include <net/if_utun.h>
#include <string.h>

#include "ifname.h"

void utun_ifname(char *name, int fd, size_t size)
{
    // Get iface name of newly created utun dev.
    char utunname[size];
    socklen_t utunname_len = (socklen_t)sizeof(utunname);
    if (getsockopt(fd, SYSPROTO_CONTROL, UTUN_OPT_IFNAME, utunname, &utunname_len))
        return;
    // name = (char *)utunname;
    strcpy(name, (const char *)utunname);
}