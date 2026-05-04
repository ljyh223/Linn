project('linn', 'c',
  version: '0.1.0',
  meson_version: '>=0.59'
)

prefix = get_option('prefix')
datadir = join_paths(prefix, get_option('datadir'))
icondir = join_paths(datadir, 'icons', 'hicolor')

gnome = import('gnome')

# =========================
# 1. Desktop 文件
# =========================
install_data(
  'data/io.github.ljyh223.Linn.desktop',
  install_dir: join_paths(datadir, 'applications')
)

# =========================
# 2. AppStream Metainfo
# =========================
install_data(
  'data/io.github.ljyh223.Linn.metainfo.xml',
  install_dir: join_paths(datadir, 'metainfo')
)

# =========================
# 3. 应用图标 (SVG → hicolor)
# =========================
install_data(
  'data/io.github.ljyh223.Linn.svg',
  install_dir: join_paths(icondir, 'scalable', 'apps'),
  rename: 'io.github.ljyh223.Linn.svg'
)

# =========================
# 4. GSettings Schema
# =========================
install_data(
  'data/io.github.ljyh223.Linn.gschema.xml',
  install_dir: join_paths(datadir, 'glib-2.0', 'schemas')
)

# Schema 验证
compile_schemas = find_program('glib-compile-schemas', required: false)
if compile_schemas.found()
  test('validate-schemas', compile_schemas,
    args: ['--strict', '--dry-run', meson.current_source_dir() / 'data']
  )
endif

# Schema 编译 + 图标缓存更新（安装后）
gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
)
