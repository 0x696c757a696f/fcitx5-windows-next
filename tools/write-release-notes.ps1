[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $ArtifactDirectory,
  [Parameter(Mandatory)] [string] $Version,
  [ValidateSet('stable', 'beta', 'nightly')] [string] $Channel = 'stable',
  [switch] $WithoutAuthenticode,
  [string] $Repository = '0x696c757a696f/fcitx5-windows-next',
  [Parameter(Mandatory)] [string] $OutputPath
)

# Writes the bilingual (Simplified Chinese / English) part of the GitHub
# Release body: what every published file is and who needs it. The
# workflow passes it with --generate-notes, and GitHub appends the change
# list after it.
# Every top-level file in the artifact directory must be described here, so
# a new asset cannot be published without an explanation.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$suffix = if ($Channel -eq 'stable') { '' } else { "-$Channel" }
$directorySuffix = switch ($Channel) { 'beta' { '-Beta' } 'nightly' { '-Nightly' } default { '' } }
$tag = "v$Version"
$downloadBase = "https://github.com/$Repository/releases/download/$tag"
$prerelease = $Channel -ne 'stable' -or $WithoutAuthenticode

$files = @{}
foreach ($file in Get-ChildItem -LiteralPath $ArtifactDirectory -File) { $files[$file.Name] = $file }
if ($files.Count -eq 0) { throw "No release artifacts in $ArtifactDirectory." }
$described = [Collections.Generic.HashSet[string]]::new()

function Format-Size([long] $bytes) {
  if ($bytes -ge 1MB) { return '{0:0.0} MiB' -f ($bytes / 1MB) }
  if ($bytes -ge 1KB) { return '{0:0.0} KiB' -f ($bytes / 1KB) }
  return "$bytes B"
}

function Get-AssetLink([string] $name) {
  if (-not $files.ContainsKey($name)) { throw "Missing release asset: $name" }
  [void] $described.Add($name)
  return "[``$name``]($downloadBase/$name)<br><sub>$(Format-Size $files[$name].Length)</sub>"
}

# Chinese text for the plugin inventories' English titles and summaries.
$titleZh = @{
  'Array 30' = '行列 30'; 'Boshiamy' = '嘸蝦米'; 'Cangjie' = '仓颉'; 'Cantonese' = '粤语'
  'Quick' = '速成'; 'Stroke' = '笔画'; 'Wu' = '吴语'; 'Wubi 86' = '五笔 86'; 'Wubi 98' = '五笔 98'
  'Zhengma' = '郑码'; 'Jyutping' = '粤拼'; 'Chewing' = '新酷音'; 'Zhuyin' = '注音'
  'Emoji' = '表情符号'; 'Russian Traditional' = '俄语传统'; 'Russian Transliteration' = '俄语转写'
  'Ukrainian Transliteration' = '乌克兰语转写'; 'Vietnamese VIQR' = '越南语 VIQR'
  'Thai Table' = '泰语码表'; 'Arabic' = '阿拉伯语'; 'Amharic' = '阿姆哈拉语'
  'Malayalam Phonetic' = '马拉雅拉姆语拼音'; 'Tamil Remington' = '泰米尔语 Remington'
  'Hangul' = '韩文'; 'Thai' = '泰语'
  'Wubi' = '五笔字型'; 'Wubi Pinyin' = '五笔拼音'; 'Cangjie (Simplified Chinese)' = '仓颉（简体中文）'
  'Erbi' = '二笔'; 'Bingchan' = '冰蟾全息'; 'Wanfeng' = '晚风'; 'Ziranma' = '自然码'
  'Dianbaoma' = '电报码'
}
$summaryZh = @{
  'Rime input engine' = 'Rime 输入法引擎（中州韵）'
  'Lua addon loader' = 'Lua 扩展加载器，供 Lua 脚本插件使用'
  'Vietnamese input method' = '越南语输入法'
  'Japanese input method' = '日语输入法'
  'Japanese SKK input method' = '日语 SKK 输入法'
  'Japanese SKK input method based on libcskk' = '日语 SKK 输入法（基于 libcskk）'
  'Japanese Kana Kanji input method' = '日语假名汉字转换输入法'
  'Traditional Chinese Zhuyin input method' = '繁体中文注音输入法'
  'Traditional Chinese Zhuyin input method based on libzhuyin' = '繁体中文注音输入法（基于 libzhuyin）'
  'English input method with word suggestions' = '带单词联想的英文输入法'
  'Korean input method' = '韩语输入法'
  'Cantonese input method' = '粤语输入法'
  'Keyman keyboard layouts' = 'Keyman 键盘布局'
  'Multilingual input methods from m17n' = '来自 m17n 的多语种输入法'
  'Sinhala input method' = '僧伽罗语输入法'
  'Thai input method based on libthai' = '泰语输入法（基于 libthai）'
  'Chinese table input method data' = '中文码表输入法'
  'Table input method data' = '码表输入法'
}
$plugins = @{}
foreach ($inventory in @('tools/release-plugin-inventory.json', 'tools/fork-plugin-inventory.json')) {
  $data = Get-Content -LiteralPath (Join-Path $repoRoot $inventory) -Raw | ConvertFrom-Json
  foreach ($package in $data.packages) { $plugins[$package.id] = $package }
}

$lines = [Collections.Generic.List[string]]::new()
function Add-Line([string] $text = '') { $lines.Add($text) }

if ($prerelease) {
  Add-Line '> [!WARNING]'
  Add-Line '> **预发布版**：这是开发者预览版，用于测试和评估，尚未完成全部发行验证。'
  Add-Line '> **Pre-release:** a developer preview for testing and evaluation; it has not passed every release gate yet.'
  if ($WithoutAuthenticode) {
    Add-Line '>'
    Add-Line '> 安装包和程序还没有 Authenticode 代码签名证书，Windows SmartScreen 可能提示**未知发布者**。可以先用下文的签名清单核对文件，再点**更多信息 → 仍要运行**。'
    Add-Line '> The installer and binaries have no Authenticode signature yet, so Windows SmartScreen may warn about an unknown publisher. Check the files against the signed manifest below, then choose **More info → Run anyway**.'
  }
  Add-Line
}

$setup = "fcitx5-windows-$Version$suffix-setup.exe"
$portable = "fcitx5-windows-$Version$suffix-portable.zip"
Add-Line '## 📥 下载哪个文件？ · Which file do I need?'
Add-Line
Add-Line "**大多数人只需要下载 [``$setup``]($downloadBase/$setup)。** 其他文件都是可选的。"
Add-Line "<br>**Most people only need [``$setup``]($downloadBase/$setup).** Everything else is optional."
Add-Line
Add-Line '系统要求 · Requirements：Windows 10 / 11，x64（同时支持 32 位应用）· x64, 32-bit apps included.'
Add-Line
Add-Line '### 🖥️ 程序 · Application'
Add-Line
Add-Line '| 文件 · File | 是什么 · What it is | 给谁用 · Who it is for |'
Add-Line '| :-- | :-- | :-- |'
Add-Line ("| $(Get-AssetLink $setup) | **安装包（推荐）**。安装到 ``Program Files\Fcitx5$directorySuffix``，注册输入法并创建开始菜单快捷方式；需要管理员权限，可在**设置 → 应用**中卸载。" +
  "<br><sub>**Installer (recommended).** Installs to ``Program Files\Fcitx5$directorySuffix``, registers the input method and adds Start menu shortcuts. Needs administrator rights; uninstall from Settings → Apps.</sub> " +
  '| 所有人<br><sub>Everyone</sub> |')
Add-Line ("| $(Get-AssetLink $portable) | **便携版**。解压到任意文件夹，运行 ``Start Fcitx5.exe``；第一次启动需要管理员批准一次来注册输入法。配置和词库保存在这个文件夹里。" +
  "<br><sub>**Portable build.** Extract anywhere and run ``Start Fcitx5.exe``; the first start asks once for administrator approval to register the input method. Settings and user dictionaries stay inside the folder.</sub> " +
  '| 不想安装、想放在 U 盘或先试用的人<br><sub>Trying it out, or running without installing</sub> |')
Add-Line

$pluginNames = @($files.Keys | Where-Object { $_ -like '*.fcpkg' } | Sort-Object)
if ($pluginNames.Count -ne 0) {
  Add-Line "### 🧩 插件包 · Plugins (``*.fcpkg``)"
  Add-Line
  Add-Line '**一般不用手动下载。** 打开 **Fcitx5 设置 → 插件与扩展**，点**安装**即可，程序会自动从这里下载并校验签名。每个插件包都带有 ML-DSA-65 + Ed25519 双重签名。'
  Add-Line '<br>**You normally do not download these by hand.** Open **Fcitx5 Settings → Add-ons & Extensions** and choose Install; it downloads the package from this release and checks its signatures. Every package is signed with both ML-DSA-65 and Ed25519.'
  Add-Line
  Add-Line "<details>"
  Add-Line "<summary><b>全部 $($pluginNames.Count) 个插件包 · All $($pluginNames.Count) plugin packages</b></summary>"
  Add-Line
  Add-Line '| 文件 · File | 输入法 · Input method | 说明 · Description |'
  Add-Line '| :-- | :-- | :-- |'
  foreach ($name in $pluginNames) {
    $id = $name -replace "-$([regex]::Escape($Version))-[^-]+\.fcpkg$", ''
    $title = $id; $zh = ''; $en = ''
    if ($plugins.ContainsKey($id)) {
      $package = $plugins[$id]
      $title = if ($titleZh.ContainsKey($package.title)) { "$($titleZh[$package.title]) · $($package.title)" } else { $package.title }
      $en = $package.summary
      $zh = if ($summaryZh.ContainsKey($en)) { $summaryZh[$en] } else { $en }
    }
    Add-Line "| $(Get-AssetLink $name) | $title | $zh<br><sub>$en</sub> |"
  }
  Add-Line
  Add-Line '</details>'
  Add-Line
}

Add-Line '### 🗂️ 插件目录 · Plugin catalog'
Add-Line
Add-Line '程序自动读取，不需要手动下载。 · Read by the application; no need to download.'
Add-Line
Add-Line '| 文件 · File | 是什么 · What it is |'
Add-Line '| :-- | :-- |'
Add-Line ("| $(Get-AssetLink 'index.json') | 插件目录：列出所有插件包的版本、大小和哈希，**插件与扩展**页面据此安装和更新。" +
  '<br><sub>Plugin index: version, size and hash of every plugin package. Add-ons & Extensions installs and updates from it.</sub> |')
Add-Line ("| $(Get-AssetLink 'index.sig.json') | 插件目录的 ML-DSA-65 + Ed25519 签名；签名不对时程序会拒绝这个目录。每天自动重新签名，避免过期。" +
  '<br><sub>ML-DSA-65 + Ed25519 signature of the index; the application rejects an index whose signature does not verify. Re-signed daily so it does not expire.</sub> |')
Add-Line

$manifest = "release-manifest-$Version$suffix.json"
Add-Line '### 🔏 校验与签名 · Verification'
Add-Line
Add-Line '想确认下载的文件没被篡改时使用。 · For checking that the downloads were not tampered with.'
Add-Line
Add-Line '| 文件 · File | 是什么 · What it is |'
Add-Line '| :-- | :-- |'
Add-Line ("| $(Get-AssetLink $manifest) | 发布清单：记录本页每个下载文件（安装包、便携版、插件包、插件目录、SBOM）的大小和 SHA-256。" +
  '<br><sub>Release manifest: size and SHA-256 of every download on this page (installer, portable build, plugins, plugin index, SBOM).</sub> |')
Add-Line ("| $(Get-AssetLink "release-manifest-$Version$suffix.sig.json") | 发布清单的 ML-DSA-65 + Ed25519 双重签名。清单签名有效，就说明清单里列出的所有文件都是官方发布的。" +
  '<br><sub>ML-DSA-65 + Ed25519 signature of the release manifest. A valid signature vouches for every file the manifest lists.</sub> |')
if ($files.ContainsKey("$manifest.p7s")) {
  Add-Line ("| $(Get-AssetLink "$manifest.p7s") | 用 Authenticode 代码签名证书对发布清单做的 CMS 分离签名。" +
    '<br><sub>Detached CMS signature of the release manifest made with the Authenticode certificate.</sub> |')
}
Add-Line ("| $(Get-AssetLink "fcitx5-windows-$Version$suffix.spdx.json") | SBOM（SPDX 软件物料清单）：程序包含的所有组件、版本和许可证。" +
  '<br><sub>SBOM in SPDX format: every component shipped, with versions and licenses.</sub> |')
Add-Line ("| $(Get-AssetLink "provenance-$Version$suffix.json") | 构建来源证明（in-toto / SLSA）：这些文件由哪个源码提交、哪个发布流程构建。" +
  '<br><sub>Build provenance (in-toto / SLSA): the source commit and release workflow that produced these files.</sub> |')
Add-Line
Add-Line '<details>'
Add-Line '<summary><b>如何校验 · How to verify</b></summary>'
Add-Line
Add-Line '```powershell'
Add-Line '# 1. 计算下载文件的 SHA-256，与发布清单里同名条目的 sha256 比较'
Add-Line '#    Hash the download and compare it with the sha256 of the same name in the release manifest'
Add-Line "Get-FileHash .\$setup -Algorithm SHA256"
Add-Line "(Get-Content .\$manifest | ConvertFrom-Json).artifacts | Where-Object name -eq '$setup'"
Add-Line
Add-Line '# 2. 安装后，用随程序附带的受信任密钥验证清单签名'
Add-Line '#    After installing, verify the manifest signature with the bundled trusted keys'
Add-Line "`$fcitx = `"`$env:ProgramFiles\Fcitx5$directorySuffix`""
Add-Line "& `"`$fcitx\bin\fcitx5-package.exe`" --verify-release-manifest .\$manifest .\release-manifest-$Version$suffix.sig.json `"`$fcitx\security\trusted-keys.json`""
Add-Line '```'
Add-Line
Add-Line '</details>'
Add-Line

$undescribed = @($files.Keys | Where-Object { -not $described.Contains($_) } | Sort-Object)
if ($undescribed.Count -ne 0) {
  throw "Release assets without a description in tools/write-release-notes.ps1: $($undescribed -join ', ')"
}

Add-Line '---'
Add-Line
[IO.File]::WriteAllText($OutputPath, (($lines -join "`n") + "`n"), [Text.UTF8Encoding]::new($false))
Write-Host "Release notes written: $OutputPath"
