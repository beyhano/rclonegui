#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$REPO_ROOT/rclone-bin"
DIST_DIR="$REPO_ROOT/build-dist"

# Renkli log yazdırma fonksiyonları
log_info() {
    echo -e "\e[34m$1\e[0m"
}
log_success() {
    echo -e "\e[32m$1\e[0m"
}
log_warn() {
    echo -e "\e[33m$1\e[0m"
}
log_error() {
    echo -e "\e[31m$1\e[0m"
}

get_version() {
    local conf="$REPO_ROOT/src-tauri/tauri.conf.json"
    if [ ! -f "$conf" ]; then
        log_error "tauri.conf.json bulunamadı!"
        exit 1
    fi
    if command -v jq &> /dev/null; then
        jq -r '.version' "$conf"
    else
        grep '"version"' "$conf" | head -n 1 | cut -d'"' -f4
    fi
}

copy_bundles_to_dist() {
    mkdir -p "$DIST_DIR"
    local bundle_dir="$REPO_ROOT/src-tauri/target/release/bundle"
    if [ -d "$bundle_dir" ]; then
        log_info "\n  Bundle çıktıları kopyalanıyor (build-dist/):"
        find "$bundle_dir" -type f \( -name "rclonegui*" -o -name "*.deb" -o -name "*.AppImage" \) | while read -r file; do
            cp "$file" "$DIST_DIR/"
            log_success "    [+] $(basename "$file") -> build-dist/"
        done
    else
        log_warn "  Bundle klasörü bulunamadı: $bundle_dir"
    fi
}

check_binaries() {
    log_info "\n=== rclone Binary Kontrolü ==="
    declare -A platforms=(
        ["windows/rclone.exe"]="Windows"
        ["linux/rclone"]="Linux"
        ["osx-amd64/rclone"]="macOS (Intel)"
        ["osx-arm64/rclone"]="macOS (ARM)"
    )
    local all_ok=true
    for rel_path in "${!platforms[@]}"; do
        local full_path="$BIN_DIR/$rel_path"
        if [ -f "$full_path" ]; then
            local size
            size=$(du -k "$full_path" | cut -f1)
            log_success "  [OK] ${platforms[$rel_path]}: $rel_path ($size KB)"
        else
            log_error "  [XX] ${platforms[$rel_path]}: $rel_path — EKSİK"
            all_ok=false
        fi
    done
    if [ "$all_ok" = false ]; then
        log_warn "  -> ./rclone-setup.sh --download"
    fi
}

download_binaries() {
    local rclone_version="$1"
    log_info "\n=== rclone Binary İndir ==="
    
    if [ "$rclone_version" = "current" ]; then
        log_info "  Son sürüm alınıyor..."
        if command -v curl &> /dev/null && command -v jq &> /dev/null; then
            rclone_version=$(curl -s "https://api.github.com/repos/rclone/rclone/releases/latest" | jq -r '.tag_name')
        else
            rclone_version=$(curl -s "https://api.github.com/repos/rclone/rclone/releases/latest" | grep '"tag_name":' | head -n 1 | cut -d'"' -f4)
        fi
        log_success "  Sürüm: $rclone_version"
    fi

    # Format: folder:file:archive_suffix
    local targets=(
        "windows:rclone.exe:windows-amd64"
        "linux:rclone:linux-amd64"
        "osx-amd64:rclone:osx-amd64"
        "osx-arm64:rclone:osx-arm64"
    )

    local temp_dir
    temp_dir=$(mktemp -d)
    
    for t in "${targets[@]}"; do
        IFS=":" read -r os_folder file_name archive_suffix <<< "$t"
        local out_dir="$BIN_DIR/$os_folder"
        local out_file="$out_dir/$file_name"
        
        mkdir -p "$out_dir"
        if [ -f "$out_file" ]; then
            log_success "  [OK] $file_name — var"
            continue
        fi
        
        local url="https://github.com/rclone/rclone/releases/download/$rclone_version/rclone-$rclone_version-$archive_suffix.zip"
        local zip_file="$temp_dir/rclone-$archive_suffix.zip"
        local extract_dir="$temp_dir/rclone-extract-$archive_suffix"
        
        log_info "  İndiriliyor: $archive_suffix..."
        if curl -L -s -o "$zip_file" "$url"; then
            unzip -q -o "$zip_file" -d "$extract_dir"
            local found
            found=$(find "$extract_dir" -type f -name "$file_name" -print -quit)
            if [ -n "$found" ]; then
                cp "$found" "$out_file"
                chmod +x "$out_file"
                log_success "  [OK] $file_name ($os_folder)"
            else
                log_error "  [XX] Arşiv içerisinde $file_name bulunamadı!"
            fi
        else
            log_error "  [XX] İndirme başarısız: $url"
        fi
    done
    rm -rf "$temp_dir"
}

build_tauri() {
    log_info "\n=== Tauri Build ==="
    local version
    version=$(get_version)
    log_info "  Sürüm: $version"
    
    log_info "  -> pnpm install"
    pnpm install
    
    log_info "  -> pnpm tauri build"
    export TAURI_SIGNING_PRIVATE_KEY="/home/beyhan/.tauri/rclonegui.key"
    export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="rcloneguikey"
    
    pnpm tauri build
    
    copy_bundles_to_dist
}

publish_release() {
    log_info "\n=== GitHub Release ==="
    local version
    version=$(get_version)
    local tag="v$version"
    log_info "  Sürüm: $version -> tag: $tag"
    
    if ! command -v gh &> /dev/null; then
        log_error "  [XX] gh CLI gerekli! Kurmak için: sudo apt install gh"
        return
    fi
    
    if ! gh auth status &> /dev/null; then
        log_error "  [XX] gh CLI oturumu açık değil. Önce 'gh auth login' yapın."
        return
    fi
    
    if [ ! -d "$DIST_DIR" ]; then
        log_error "  [XX] build-dist/ klasörü bulunamadı! Önce build işlemini çalıştırın."
        return
    fi
    
    local assets=()
    while IFS= read -r -d '' file; do
        assets+=("$file")
    done < <(find "$DIST_DIR" -type f \( -name "rclonegui*" -o -name "*.deb" -o -name "*.AppImage" \) -print0)
    
    if [ ${#assets[@]} -eq 0 ]; then
        log_error "  [XX] Yüklenecek paket bulunamadı!"
        return
    fi
    
    # Tag at ve pushla
    log_info "  -> git tag $tag"
    git tag -f "$tag"
    log_info "  -> git push origin $tag"
    git push origin "$tag" --force
    
    log_info "  -> gh release create $tag"
    gh release create "$tag" "${assets[@]}" --title "rclonegui $version" --notes "rclonegui $version"
    
    log_success "\n  [OK] Release oluşturuldu!"
    log_info "  https://github.com/beyhano/rclonegui/releases/tag/$tag"
}

publish_git() {
    log_info "\n=== GitHub CI/CD Release ==="
    local version
    version=$(get_version)
    local tag="v$version"
    log_info "  Sürüm: $version -> Etiket: $tag"
    
    local default_msg="release: $tag"
    echo -e "  Commit mesajı girin (Boş bırakırsanız '$default_msg' kullanılacak): \c"
    read -r msg
    if [ -z "$msg" ]; then
        msg="$default_msg"
    fi
    
    log_info "  -> git add ."
    git add .
    
    log_info "  -> git commit -m '$msg'"
    git commit -m "$msg"
    
    log_info "  -> git push origin main"
    git push origin main
    
    # Tag kontrolü
    if git tag -l | grep -q "^$tag$"; then
        echo -e "  [!] $tag etiketi zaten mevcut. Yeniden oluşturulsun mu? (Y = evet, N = hayır): \c"
        read -r overwrite
        if [[ "$overwrite" =~ ^[Yy]$ ]]; then
            git tag -d "$tag"
            git push origin --delete "$tag" || true
        else
            log_error "  [XX] İşlem iptal edildi."
            return
        fi
    fi
    
    log_info "  -> git tag $tag"
    git tag "$tag"
    
    log_info "  -> git push origin $tag"
    git push origin "$tag"
    
    log_success "\n  [OK] GitHub Actions tetiklendi! Derleme süreci GitHub üzerinden takip edilebilir."
}

# Main
log_info "=============================="
log_info " rclonegui - Linux Build Helper"
log_info "=============================="

CHECK=false
DOWNLOAD=false
TAURI_BUILD=false
RELEASE=false
PUBLISH=false
RCLONE_VERSION="current"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --check) CHECK=true; shift ;;
        --download) DOWNLOAD=true; shift ;;
        --tauri-build) TAURI_BUILD=true; shift ;;
        --release) RELEASE=true; shift ;;
        --publish) PUBLISH=true; shift ;;
        --rclone-version) RCLONE_VERSION="$2"; shift 2 ;;
        *) log_error "Bilinmeyen parametre: $1"; exit 1 ;;
    esac
done

if [ "$CHECK" = false ] && [ "$DOWNLOAD" = false ] && [ "$TAURI_BUILD" = false ] && [ "$RELEASE" = false ] && [ "$PUBLISH" = false ]; then
    version=$(get_version)
    log_info "  Sürüm: $version\n"
    log_info "  Kullanım:"
    log_info "    ./rclone-setup.sh --check          # Binary kontrol"
    log_info "    ./rclone-setup.sh --download       # rclone indir"
    log_info "    ./rclone-setup.sh --tauri-build    # Yerel Tauri derle (.deb + .AppImage)"
    log_info "    ./rclone-setup.sh --release        # Yerel GitHub Release"
    log_info "    ./rclone-setup.sh --publish        # GitHub Actions CI/CD Release tetikle"
    exit 0
fi

if [ "$CHECK" = true ]; then check_binaries; fi
if [ "$DOWNLOAD" = true ]; then download_binaries "$RCLONE_VERSION"; fi
if [ "$TAURI_BUILD" = true ]; then build_tauri; fi
if [ "$RELEASE" = true ]; then publish_release; fi
if [ "$PUBLISH" = true ]; then publish_git; fi
