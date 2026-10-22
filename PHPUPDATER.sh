#!/usr/bin/env bash
# PHP updater for debian - closes PHP2Shell (CVE-2026-109618 + CVE-2026-109619)
# unauth remote code execution in php-fpm / apache mod_php / php-cgi, fixed by php.net on 2026-10-21
#
#  - every installed php package gets updated automatically if apt has an update (always, no question)
#  - restarts php-fpm / apache2 if they still run the old php
#  - checks if a vulnerable php is still there, if yes you get asked: update / remove / skip
#
# usage: sudo bash PHPUPDATER.sh       update + check
#        bash PHPUPDATER.sh --check    only show the status, changes nothing
# exit:  0 = no vulnerable php left, 1 = vulnerable php still there, 2 = error

set -u
export LC_ALL=C DEBIAN_FRONTEND=noninteractive NEEDRESTART_SUSPEND=1

CVE1=CVE-2026-109618
CVE2=CVE-2026-109619
ADVISORY="https://www.php.net/archive/2026.php#2026-10-21-5"
# first fixed version per branch, 5.4 - 8.1 are end of life and got no release
declare -A FIXED=([8.2]=8.2.35 [8.3]=8.3.36 [8.4]=8.4.27 [8.5]=8.5.12)
FIRST_AFFECTED=5.4
NEWEST_KNOWN=8.5
BACKUP=/var/backups/phpupdater-$(date +%Y%m%d-%H%M%S)
APT=(apt-get -y -q -o DPkg::Lock::Timeout=300 -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold)
PHPRE='^((lib(apache2-mod-)?)?php|lsphp)'
NAMERE='^(lib(apache2-mod-)?)?php([0-9]+\.[0-9]+)(-|$)'
SRCRE='^php([0-9]+\.[0-9]+)$'
BINRE='^(ls)?php(-fpm|-cgi)?[0-9.]*$'

if [ -t 1 ]; then R=$'\e[31m' G=$'\e[32m' Y=$'\e[33m' B=$'\e[1m' N=$'\e[0m'; else R='' G='' Y='' B='' N=''; fi
ok()   { echo "${G}[ok]${N} $*"; }
warn() { echo "${Y}[!]${N} $*"; }
bad()  { echo "${R}[vulnerable]${N} $*"; }
die()  { echo "${R}error:${N} $*" >&2; exit 2; }

CHECK=0
case "${1:-}" in
    "") ;;
    --check) CHECK=1 ;;
    -h|--help) sed -n '2,11p' "$0" | cut -c3-; exit 0 ;;
    *) die "unknown option: $1 (only --check)" ;;
esac

[ -f /etc/debian_version ] && command -v apt-get >/dev/null && command -v dpkg-query >/dev/null ||
    die "this is for debian (apt + dpkg) only."

if [ "$CHECK" = 0 ] && [ "$(id -u)" != 0 ]; then
    [ -f "$0" ] && command -v sudo >/dev/null || die "run it as root: sudo bash PHPUPDATER.sh"
    exec sudo bash "$0" "$@"
fi

# questions come from the terminal, without one (cron, pipe) nothing gets asked + nothing removed
TTY=""
[ "$CHECK" = 0 ] && (exec </dev/tty) 2>/dev/null && TTY=/dev/tty
ask()         { REPLY=""; [ -n "$TTY" ] && read -r -p "$1" REPLY <"$TTY"; }
confirm()     { ask "$1" && [[ ${REPLY,,} == y* ]]; }
confirm_yes() { ask "$1" && [[ -z $REPLY || ${REPLY,,} == y* ]]; }

is_debian()      { [ "$(. /etc/os-release 2>/dev/null && echo "${ID:-}")" = debian ]; }
sury_configured() { grep -rqsE '^[^#]*packages\.sury\.org/php' /etc/apt/sources.list /etc/apt/sources.list.d/; }

have_systemd() { [ -d /run/systemd/system ]; }
svc_active()   { if have_systemd; then systemctl is-active --quiet "$1"; else service "$1" status >/dev/null 2>&1; fi; }
svc_do()       { if have_systemd; then systemctl "$1" "$2"; else service "$2" "$1"; fi; }

# PKGS = all installed php packages, CORE[pkg]=version + BR[pkg]=branch for the php itself (built from the phpX.Y source)
declare -A CORE=() BR=() VULN=()
scan() {
    PKGS=(); HELD=(); CORE=(); BR=()
    local st name ver src
    while read -r st name ver src; do
        [[ $st == [hi]i* ]] || continue
        [[ $name =~ $PHPRE || ${src:-} == php* ]] || continue
        PKGS+=("$name")
        [[ $st == h* ]] && HELD+=("$name")
        # old dpkg has no source field
        [ -z "$src" ] && [[ $name =~ $NAMERE ]] && src=php${BASH_REMATCH[3]}
        [[ $src =~ $SRCRE ]] || continue
        CORE[$name]=$ver
        BR[$name]=${BASH_REMATCH[1]}
    done < <(dpkg-query -W -f='${db:Status-Abbrev} ${binary:Package} ${Version} ${source:Package}\n' 2>/dev/null)
}

# is_fixed <branch> <version> [installed pkg] -> 0 = fixed / not affected, 1 = vulnerable
is_fixed() {
    local b=$1 up=${2#*:} cl
    up=${up%%-*}
    dpkg --compare-versions "$b" lt "$FIRST_AFFECTED" && return 0
    dpkg --compare-versions "$b" gt "$NEWEST_KNOWN" && return 0
    [ -n "${FIXED[$b]:-}" ] && dpkg --compare-versions "$up" ge "${FIXED[$b]}" && return 0
    # debian lts / ubuntu / sury eol builds patch the old version, they list the cves in the changelog
    if [ -n "${3:-}" ]; then
        cl=/usr/share/doc/${3%%:*}/changelog.Debian.gz
        [ -f "$cl" ] && zgrep -qF "$CVE1" "$cl" && zgrep -qF "$CVE2" "$cl" && return 0
    fi
    return 1
}

candidate() { apt-cache policy "$1" 2>/dev/null | awk '/Candidate:/ {print $2; exit}'; }

# apt-get update, shows warnings / errors, SURY_BAD = why the sury repo cant be used (expired key = no php updates from there)
SURY_BAD=""
apt_update() {
    local out
    out=$(apt-get -q -o DPkg::Lock::Timeout=300 update 2>&1) || warn "apt-get update had errors:"
    grep -E '^(W|E):' <<<"$out" | sed 's/^/  /'
    SURY_BAD=""
    out=$(grep -E '^(W|E):.*packages\.sury\.org' <<<"$out")
    case $out in
        "") ;;
        *EXPKEYSIG*|*KEYEXPIRED*) SURY_BAD="its signing key expired" ;;
        *NO_PUBKEY*) SURY_BAD="its signing key is missing" ;;
        *Signed-By*) SURY_BAD="it is set up twice with different keys" ;;
        *"not signed"*|*signature*) SURY_BAD="its signature is invalid" ;;
    esac
}

# official sury setup (https://packages.sury.org/php/README.txt): keyring package + php.list with signed-by
# old sury entries get disabled (otherwise apt: conflicting Signed-By), backup in $BACKUP
fix_sury() {
    local codename deb kr f
    codename=$(. /etc/os-release 2>/dev/null && echo "${VERSION_CODENAME:-}")
    [ -n "$codename" ] || { warn "debian codename not found in /etc/os-release."; return 1; }
    command -v curl >/dev/null || "${APT[@]}" install ca-certificates curl >/dev/null || { warn "could not install curl."; return 1; }
    deb=$(mktemp --suffix=.deb) || return 1
    if ! curl -fsSL -o "$deb" https://packages.sury.org/debsuryorg-archive-keyring.deb ||
       [ "$(dpkg-deb -f "$deb" Package 2>/dev/null)" != debsuryorg-archive-keyring ]; then
        rm -f "$deb"; warn "download of the sury keyring failed."; return 1
    fi
    dpkg -i "$deb" >/dev/null; f=$?
    rm -f "$deb"
    [ "$f" = 0 ] || { warn "install of the sury keyring failed."; return 1; }
    kr=$(dpkg -L debsuryorg-archive-keyring 2>/dev/null | grep -m1 '^/usr/share/keyrings/.*\.gpg$')
    [ -n "$kr" ] || { warn "sury keyring file not found."; return 1; }
    for f in /etc/apt/sources.list /etc/apt/sources.list.d/*.list /etc/apt/sources.list.d/*.sources; do
        [ -f "$f" ] && grep -qE '^[^#]*packages\.sury\.org/php' "$f" || continue
        if [[ $f == *.sources ]] && grep -E '^URIs:' "$f" | grep -vq 'packages\.sury\.org'; then
            warn "$f has sury + other repos in it, remove the sury part yourself."; continue
        fi
        mkdir -p "$BACKUP${f%/*}" && cp -p "$f" "$BACKUP$f" || return 1
        if [[ $f == *.sources ]]; then rm -f "$f"; else sed -i -E '/^[[:space:]]*deb.*packages\.sury\.org\/php/ s/^/# disabled by phpupdater: /' "$f"; fi
        echo "  old sury entry disabled: $f (backup in $BACKUP)"
    done
    echo "deb [signed-by=$kr] https://packages.sury.org/php/ $codename main" >/etc/apt/sources.list.d/php.list ||
        { warn "could not write /etc/apt/sources.list.d/php.list"; return 1; }
    ok "sury repo set up again: /etc/apt/sources.list.d/php.list"
}

# updates every php package that has an update in apt (not the held ones), never removes anything
auto_update() {
    local p plan free=()
    scan
    for p in "${PKGS[@]}"; do [[ " ${HELD[*]} " == *" $p "* ]] || free+=("$p"); done
    [ ${#HELD[@]} -gt 0 ] && warn "on hold (apt-mark hold), not auto updated: ${HELD[*]}"
    if [ ${#free[@]} -gt 0 ]; then
        if ! plan=$(apt_plan install --only-upgrade --no-remove "${free[@]}"); then
            warn "auto update not possible:"; echo "$plan"
        elif [ -z "$plan" ]; then
            ok "php packages are up to date."
        else
            echo "Updating:"; echo "$plan"; echo
            upgrade_pkgs --no-remove "${free[@]}" && ok "php packages updated." || warn "update failed, see the apt output above."
        fi
    fi
    scan
    restart_stale
}

# fixed php branches newer than <branch> that apt can install (no ~rc / ~beta builds)
fixed_targets() {
    local p b c
    for p in $(apt-cache pkgnames php 2>/dev/null | grep -E '^php[0-9]+\.[0-9]+-common$' | sort -V); do
        b=${p#php}; b=${b%-common}
        dpkg --compare-versions "$b" gt "$1" || continue
        c=$(candidate "$p")
        [ -n "$c" ] && [ "$c" != "(none)" ] && [[ ${c%%-*} != *~* ]] && is_fixed "$b" "$c" && echo "$b"
    done
}

# what apt would do: "  pkg  old -> new" / "  pkg  (new) ver" / "  pkg  REMOVE", returns 1 + the apt error if it cant
apt_plan() {
    local out
    out=$(apt-get -s -o Debug::NoLocking=1 "$@" 2>&1) || { tail -n 15 <<<"$out" | sed 's/^/  /'; return 1; }
    awk '/^Inst / { if ($3 ~ /^\[/) { o=$3; gsub(/\[|\]/,"",o); n=$4; sub(/^\(/,"",n); print "  " $2 "  " o " -> " n }
                    else { n=$3; sub(/^\(/,"",n); print "  " $2 "  (new) " n } }
         /^Remv / { print "  " $2 "  REMOVE" }' <<<"$out"
}

# apt-get install --only-upgrade <option> <pkgs>, auto installed packages stay auto (apt would mark them manual)
upgrade_pkgs() {
    local opt=$1 auto rc
    shift
    auto=$(apt-mark showauto "$@" 2>/dev/null)
    "${APT[@]}" install --only-upgrade "$opt" "$@"
    rc=$?
    [ -n "$auto" ] && apt-mark auto $auto >/dev/null
    return $rc
}

# every installed package of php<branch>, also extensions from other sources (php8.2-redis ...)
branch_pkgs() {
    local p re="^(lib(apache2-mod-)?)?php${1//./\\.}(-|$)"
    for p in "${PKGS[@]}"; do [[ $p =~ $re || ${BR[$p]:-} == "$1" ]] && echo "$p"; done
}

# the package that stands for php<branch>
branch_main() {
    local p
    [ -n "${CORE[php$1-common]:-}" ] && { echo "php$1-common"; return; }
    for p in "${!CORE[@]}"; do [ "${BR[$p]}" = "$1" ] && { echo "$p"; return; }; done
}

sapis() {
    local s=()
    [ -n "${CORE[php$1-fpm]:-}" ] && s+=(fpm)
    [ -n "${CORE[libapache2-mod-php$1]:-}" ] && s+=("apache mod_php")
    [ -n "${CORE[php$1-cgi]:-}" ] && s+=(cgi)
    [ -n "${CORE[php$1-cli]:-}" ] && s+=(cli)
    [ ${#s[@]} -gt 0 ] || s=("no sapi")
    local IFS=,; echo "${s[*]}" | sed 's/,/, /g'
}

still_vuln() {
    local p
    scan
    for p in "${!CORE[@]}"; do [ "${BR[$p]}" = "$1" ] && ! is_fixed "$1" "${CORE[$p]}" "$p" && return 0; done
    return 1
}

# pids that still run php files apt replaced / removed (old php stays in memory until restart)
stale() {
    local m
    for m in /proc/[0-9]*/maps; do
        grep -qE '(/usr/s?bin/(ls)?php|/libphp|/usr/lib/php/)[^ ]* \(deleted\)$' "$m" 2>/dev/null || continue
        m=${m#/proc/}; echo "${m%/maps}"
    done
}

# pid -> its service (systemd unit, without systemd from the process name)
svc_of() {
    local u c
    u=$(grep -oE 'system\.slice/[^/]+\.service$' "/proc/$1/cgroup" 2>/dev/null | head -n1)
    if [ -n "$u" ]; then u=${u##*/}; echo "${u%.service}"; return; fi
    c=$(cat "/proc/$1/comm" 2>/dev/null)
    case $c in
        php-fpm*) echo "php${c#php-fpm}-fpm" ;;
        apache2) echo apache2 ;;
    esac
}

# restarts php-fpm / apache2 if they still run the old php, asks for other services
restart_stale() {
    local pid s svcs=() other=()
    [ "$CHECK" = 0 ] || return 0
    while read -r pid; do
        s=$(svc_of "$pid")
        [ -n "$s" ] && [[ " ${svcs[*]} " != *" $s "* ]] && svcs+=("$s")
    done < <(stale)
    for s in "${svcs[@]}"; do
        case $s in
            php*-fpm|apache2)
                if [ "$s" = apache2 ] && ! apache2ctl configtest >/dev/null 2>&1; then
                    warn "apache2 config test failed, not restarted (apache2ctl configtest)"; continue
                fi
                svc_do restart "$s" >/dev/null 2>&1 && ok "restarted $s" || warn "restart of $s failed" ;;
            *) other+=("$s") ;;
        esac
    done
    if [ ${#other[@]} -gt 0 ]; then
        warn "these services still run the old php: ${other[*]}"
        if confirm "  restart them now? [y/N]: "; then
            for s in "${other[@]}"; do svc_do restart "$s" >/dev/null 2>&1 && ok "restarted $s" || warn "restart of $s failed"; done
        fi
    fi
}

# php that apt doesnt manage (compiled / copied / litespeed), apt cant update these
MANUAL_VULN=0
manual_php() {
    local f real own v b
    local -A seen=()
    for f in $(type -ap php php-fpm php-cgi 2>/dev/null) /usr/local/bin/* /usr/local/sbin/* /opt/*/bin/* /opt/*/sbin/* /usr/local/lsws/lsphp*/bin/*; do
        [[ ${f##*/} =~ $BINRE ]] || continue
        real=$(readlink -f "$f") && [ -x "$real" ] && [ -z "${seen[$real]:-}" ] || continue
        seen[$real]=1
        [ "$(head -c4 "$real" 2>/dev/null)" = $'\x7fELF' ] || continue
        own=$(dpkg -S "$real" 2>/dev/null | cut -d: -f1)
        [ -n "$own" ] && [[ $own != lsphp* ]] && continue
        v=$(timeout 10 "$real" -v 2>/dev/null | grep -oE '^PHP [0-9]+\.[0-9]+\.[0-9]+' | head -n1)
        [ -n "$v" ] || continue
        v=${v#PHP }; b=${v%.*}
        if is_fixed "$b" "$v"; then
            ok "PHP $v  $real  (${own:-not from apt})"
        else
            bad "PHP $v  $real  (${own:-not from apt}) - apt cant fix this one, update / rebuild it yourself, then restart it"
            MANUAL_VULN=1
        fi
    done
}

# newer version of the same branch is in apt (package on hold / auto update failed)
update_branch() {
    local b=$1 plan pkgs
    mapfile -t pkgs < <(branch_pkgs "$b")
    plan=$(apt_plan install --only-upgrade --allow-change-held-packages "${pkgs[@]}") || { warn "apt cant update php$b:"; echo "$plan"; return 1; }
    echo "$plan"
    if grep -q REMOVE <<<"$plan"; then confirm "  apt also removes packages (see above), continue? [y/N]: " || return 1; fi
    upgrade_pkgs --allow-change-held-packages "${pkgs[@]}" || return 1
    restart_stale
}

# remove_branch <branch> [auto] - auto = only ask if apt removes more than php<branch>
remove_branch() {
    local b=$1 plan extra pkgs
    mapfile -t pkgs < <(branch_pkgs "$b")
    plan=$(apt_plan remove --allow-change-held-packages "${pkgs[@]}") || { warn "apt cant remove php$b:"; echo "$plan"; return 1; }
    echo "  gets removed:"; echo "$plan"
    extra=$(awk '/REMOVE$/ {print $1}' <<<"$plan" | grep -vxF -f <(printf '%s\n' "${pkgs[@]}"))
    if [ "${2:-}" != auto ] || [ -n "$extra" ]; then
        [ -n "$extra" ] && warn "apt also removes other packages: $(echo $extra)"
        [ -e "/etc/apache2/mods-enabled/php$b.load" ] || [ -e "/etc/apache2/conf-enabled/php$b-fpm.conf" ] &&
            warn "apache uses php$b! without it apache sends .php files as download (source code + passwords). only remove it if no website uses it."
        confirm "  remove? [y/N]: " || return 1
    fi
    "${APT[@]}" remove --allow-change-held-packages "${pkgs[@]}" || return 1
    scan
    restart_stale
    ok "php$b removed (configs in /etc/php/$b are kept)"
}

# moves the webserver from php<old> to php<new>: apache mod / fpm conf, own fpm pools, fpm sockets in nginx + apache configs
move_webserver() {
    local old=$1 new=$2 re=${1//./\\.} f t mod=0 conf=0 nginx_ok=0 apache_ok=0 fail=0 changed=() copied=()
    command -v nginx >/dev/null && nginx -t >/dev/null 2>&1 && nginx_ok=1
    command -v apache2ctl >/dev/null && apache2ctl configtest >/dev/null 2>&1 && apache_ok=1

    # apache without php sends .php files as download (source code), so php<new> has to be there before php<old> goes
    if [ -e "/etc/apache2/mods-enabled/php$old.load" ]; then
        [ -e "/etc/apache2/mods-available/php$new.load" ] || { warn "apache uses mod_php$old, but mod_php$new is missing. stopped."; return 1; }
        mod=1; a2dismod -q "php$old" >/dev/null; a2enmod -q "php$new" >/dev/null; echo "  apache: mod_php$old -> mod_php$new"
        [ -e "/etc/apache2/mods-enabled/php$new.load" ] || { fail=1; warn "a2enmod php$new failed"; }
    fi
    if [ -e "/etc/apache2/conf-enabled/php$old-fpm.conf" ]; then
        [ -e "/etc/apache2/conf-available/php$new-fpm.conf" ] || { warn "apache uses php$old-fpm, but php$new-fpm.conf is missing. stopped."; return 1; }
        conf=1; a2disconf -q "php$old-fpm" >/dev/null; a2enconf -q "php$new-fpm" >/dev/null; echo "  apache: php$old-fpm -> php$new-fpm"
        [ -e "/etc/apache2/conf-enabled/php$new-fpm.conf" ] || { fail=1; warn "a2enconf php$new-fpm failed"; }
    fi
    # own pools, www.conf comes with the new package
    for f in /etc/php/"$old"/fpm/pool.d/*.conf; do
        t=/etc/php/$new/fpm/pool.d/${f##*/}
        [ -f "$f" ] && [ -d "${t%/*}" ] && [ ! -e "$t" ] || continue
        sed "s/php$re/php$new/g; s#/php/$re/#/php/$new/#g" "$f" >"$t" && copied+=("$t") && echo "  fpm: pool ${f##*/} copied to php$new"
    done
    while IFS= read -r f; do
        [[ $f == */conf-available/php*-fpm.conf || $f == *.dpkg-* ]] && continue
        mkdir -p "$BACKUP${f%/*}" && cp -p "$f" "$BACKUP$f" && sed -i "s/php$re-fpm/php$new-fpm/g" "$f" && changed+=("$f") &&
            echo "  socket: $f -> php$new-fpm (backup in $BACKUP)"
    done < <(grep -rlE "php$re-fpm" /etc/nginx /etc/apache2 2>/dev/null)

    if [ "$nginx_ok" = 1 ] && ! nginx -t >/dev/null 2>&1; then fail=1; warn "nginx config test failed after the change (nginx -t)"; fi
    if [ "$apache_ok" = 1 ] && ! apache2ctl configtest >/dev/null 2>&1; then fail=1; warn "apache2 config test failed after the change"; fi
    if [ ${#copied[@]} -gt 0 ] && [ -x "/usr/sbin/php-fpm$new" ] && ! "/usr/sbin/php-fpm$new" -t >/dev/null 2>&1; then
        fail=1; warn "php-fpm$new config test failed with the copied pools (php-fpm$new -t)"
    fi
    if [ "$fail" = 1 ]; then
        for f in "${changed[@]}"; do cp -p "$BACKUP$f" "$f"; done
        for f in "${copied[@]}"; do rm -f "$f"; done
        [ "$mod" = 1 ] && { a2dismod -q "php$new"; a2enmod -q "php$old"; } >/dev/null
        [ "$conf" = 1 ] && { a2disconf -q "php$new-fpm"; a2enconf -q "php$old-fpm"; } >/dev/null
        warn "changes rolled back."
        return 1
    fi

    [ -x "/usr/sbin/php-fpm$new" ] && svc_do restart "php$new-fpm" >/dev/null 2>&1
    svc_active nginx && svc_do reload nginx >/dev/null 2>&1
    svc_active apache2 && svc_do restart apache2 >/dev/null 2>&1
    echo "  note: own settings in /etc/php/$old (php.ini, conf.d, pool.d/www.conf) are not moved, compare them with /etc/php/$new"
    echo "        scripts / cronjobs that call php$old directly have to be changed by you"
}

# apt has no fixed php<old>: install a fixed branch with the same modules, move the webserver over, remove php<old>
migrate_branch() {
    local old=$1 new def p n plan targets=() olds=() inst=() miss=()
    mapfile -t targets < <(fixed_targets "$old")
    [ ${#targets[@]} -gt 0 ] || { warn "apt has no fixed php version."; return 1; }
    # default = what "apt install php" takes (debian / sury default)
    def=$(apt-cache depends php 2>/dev/null | grep -oE 'php[0-9]+\.[0-9]+' | head -n1)
    def=${def#php}
    [[ " ${targets[*]} " == *" $def "* ]] || def=${targets[-1]}
    ask "  switch to which php? (${targets[*]}) [$def]: " || return 1
    new=${REPLY:-$def}
    [[ " ${targets[*]} " == *" $new "* ]] || { warn "$new is not in the list."; return 1; }

    mapfile -t olds < <(branch_pkgs "$old")
    for p in "${olds[@]}"; do
        n=${p%%:*}; n=${n/php$old/php$new}
        if apt-get -s -o Debug::NoLocking=1 install "$n" >/dev/null 2>&1; then inst+=("$n"); else miss+=("$n"); fi
    done
    for p in "php$new-fpm" "libapache2-mod-php$new"; do
        [[ " ${miss[*]} " == *" $p "* ]] && { warn "$p is not available, the webserver would lose php. stopped."; return 1; }
    done
    plan=$(apt_plan install "${inst[@]}") || { warn "apt cant install php$new:"; echo "$plan"; return 1; }
    echo "  gets installed:"; echo "$plan"
    [ ${#miss[@]} -gt 0 ] && echo "  not available for php$new (skipped): ${miss[*]}"
    echo "  then the webserver gets moved to php$new and php$old gets removed"
    confirm "  continue? [y/N]: " || return 1
    "${APT[@]}" install "${inst[@]}" || { warn "install of php$new failed."; return 1; }
    move_webserver "$old" "$new" || { warn "php$new is installed, but php$old was NOT removed."; return 1; }
    scan
    remove_branch "$old" auto
}

fix_branch() {
    local b=$1 p c inst upd mode targets
    while still_vuln "$b"; do
        # any vulnerable package of the branch with a newer version in apt -> normal update
        upd=""
        for p in "${!CORE[@]}"; do
            [ "${BR[$p]}" = "$b" ] && ! is_fixed "$b" "${CORE[$p]}" "$p" || continue
            inst=${CORE[$p]}
            c=$(candidate "$p")
            [ -n "$c" ] && [ "$c" != "(none)" ] && dpkg --compare-versions "$c" gt "$inst" && upd=$c
        done
        targets=""
        [ -n "$upd" ] || targets=$(fixed_targets "$b" | tr '\n' ' ')
        echo
        bad "PHP $b ($inst) - $(sapis "$b")"
        if [ -n "$upd" ]; then
            mode=update; echo "  u = update php$b to $upd"
        elif [ -n "$targets" ]; then
            mode=switch; echo "  u = update - apt has no fixed php$b, so: switch to a fixed php ($targets) + remove php$b"
        elif is_debian && { [ -n "$SURY_BAD" ] || ! sury_configured; }; then
            mode=sury
            if [ -n "$SURY_BAD" ]; then echo "  apt has no fixed php because the sury repo is broken ($SURY_BAD)."
            else echo "  apt has no fixed php, debian itself doesnt ship one for php$b."; fi
            echo "  u = set up the sury php repo (packages.sury.org, official way) + update"
        else
            mode=""; echo "  apt has no fixed php for this system, updating is not possible right now."
        fi
        echo "  r = remove php$b"
        echo "  s = skip (stays vulnerable)"
        ask "  [${mode:+u/}r/s]: " || return
        case ${REPLY,,} in
            u) case $mode in
                   update) update_branch "$b" ;;
                   switch) migrate_branch "$b" ;;
                   sury) fix_sury && apt_update && auto_update ;;
               esac ;;
            r) remove_branch "$b" ;;
            s) return ;;
        esac
    done
    ok "php$b is fixed / gone."
}


echo "${B}PHP updater${N} - PHP2Shell ($CVE1 + $CVE2)"
echo

scan
if [ "$CHECK" = 0 ] && [ ${#PKGS[@]} -gt 0 ]; then
    echo "Updating package lists..."
    apt_update
    if [ -n "$SURY_BAD" ]; then
        warn "the sury php repo cant be used ($SURY_BAD), so apt gets no php updates from there."
        if [ -n "$TTY" ]; then
            confirm_yes "  repair it now (official way: keyring package + /etc/apt/sources.list.d/php.list)? [Y/n]: " && fix_sury && apt_update
        else
            echo "  run this script in a terminal to repair it, or see https://packages.sury.org/php/README.txt"
        fi
    fi
    auto_update
elif [ ${#PKGS[@]} -gt 0 ]; then
    plan=$(apt_plan install --only-upgrade "${PKGS[@]}") && [ -n "$plan" ] &&
        { echo "Updates available (from the last apt-get update):"; echo "$plan"; }
fi

VULN=()
for p in "${!CORE[@]}"; do is_fixed "${BR[$p]}" "${CORE[$p]}" "$p" || VULN[${BR[$p]}]+="$p "; done

echo
echo "${B}Status${N}"
[ ${#CORE[@]} -gt 0 ] || echo "  no php installed via apt."
for b in $(printf '%s\n' "${BR[@]}" | sort -uV); do
    p=$(branch_main "$b")
    [ -n "${VULN[$b]:-}" ] && p=${VULN[$b]%% *}
    line="PHP $b  ${CORE[$p]}  ($(sapis "$b"))"
    if [ -n "${VULN[$b]:-}" ]; then bad "$line"; else ok "$line"; fi
    [ -z "${FIXED[$b]:-}" ] && dpkg --compare-versions "$b" lt "$NEWEST_KNOWN" &&
        warn "    PHP $b is end of life, php.net doesnt fix it anymore, switch to a newer one"
done
manual_php

if [ "$CHECK" = 0 ] && [ ${#VULN[@]} -gt 0 ]; then
    if [ -n "$TTY" ]; then
        for b in $(printf '%s\n' "${!VULN[@]}" | sort -V); do fix_branch "$b"; done
    else
        warn "no terminal: vulnerable php was not touched, run this in a terminal to update / remove it."
    fi
fi

scan
left=0
for p in "${!CORE[@]}"; do is_fixed "${BR[$p]}" "${CORE[$p]}" "$p" || left=1; done
[ "$MANUAL_VULN" = 1 ] && left=1
if [ "$(id -u)" = 0 ]; then
    pids=$(stale)
    if [ -n "$pids" ]; then
        left=1
        echo
        warn "these processes still run the old php in memory, restart them:"
        for pid in $pids; do echo "  pid $pid  $(tr '\0' ' ' <"/proc/$pid/cmdline" 2>/dev/null | cut -c1-100)"; done
    fi
else
    warn "not root: running php processes were not checked"
fi

echo
if [ "$left" = 0 ]; then
    ok "${B}no vulnerable php left.${N}"
    exit 0
fi
bad "${B}vulnerable php is still there.${N} no config change fully fixes it, only updating / removing does."
echo "  $ADVISORY"
exit 1
