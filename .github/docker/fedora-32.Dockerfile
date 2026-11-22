FROM fedora:32

# Fedora 32 packages have moved from the normal mirrors to the archive.
# Keep package signature verification enabled using the image's Fedora key.
RUN printf '%s\n' \
      '[fedora-archive]' \
      'name=Fedora 32 archive' \
      'baseurl=https://archives.fedoraproject.org/pub/archive/fedora/linux/releases/32/Everything/$basearch/os/' \
      'enabled=0' \
      'gpgcheck=1' \
      'gpgkey=file:///etc/pki/rpm-gpg/RPM-GPG-KEY-fedora-32-$basearch' \
      '[updates-archive]' \
      'name=Fedora 32 updates archive' \
      'baseurl=https://archives.fedoraproject.org/pub/archive/fedora/linux/updates/32/Everything/$basearch/' \
      'enabled=0' \
      'gpgcheck=1' \
      'gpgkey=file:///etc/pki/rpm-gpg/RPM-GPG-KEY-fedora-32-$basearch' \
      > /etc/yum.repos.d/texted-archive.repo \
    && dnf --disablerepo='*' --enablerepo=fedora-archive,updates-archive \
      install --assumeyes \
        ca-certificates curl gcc gcc-c++ glibc-devel make \
        openssl-devel pkgconf-pkg-config tar gzip \
    && dnf clean all
