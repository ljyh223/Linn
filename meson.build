project('linn', 'c',
  version: '0.1.0',
  meson_version: '>=0.59'
)

# 安装路径
prefix = get_option('prefix')
datadir = get_option('datadir')

pkgdatadir = join_paths(prefix, datadir, meson.project_name())

gnome = import('gnome')
i18n = import('i18n')

# =========================
# 1. GResource（UI资源）
# =========================
resources = gnome.compile_resources(
  'linn-resources',
  'data/icons.gresource.xml',
  source_dir: 'data',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir
)

# =========================
# 2. Desktop 文件
# =========================
desktop_file = install_data(
  'data/io.github.ljyh223.Linn.desktop',
  install_dir: join_paths(datadir, 'applications')
)

# =========================
# 3. AppStream
# =========================
appstream_file = install_data(
  'data/io.github.ljyh223.Linn.metainfo.xml',
  install_dir: join_paths(datadir, 'metainfo')
)

# =========================
# 4. GSettings Schema（关键）
# =========================
install_data(
  'data/io.github.ljyh223.Linn.gschema.xml',
  install_dir: join_paths(datadir, 'glib-2.0/schemas')
)

compile_schemas = find_program('glib-compile-schemas', required: false)

if compile_schemas.found()
  test('validate schemas', compile_schemas,
    args: ['--strict', '--dry-run', 'data']
  )
endif
