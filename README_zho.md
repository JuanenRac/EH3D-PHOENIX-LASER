<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | 🇨🇳 <b>中文</b> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### 从图像进行激光雕刻，从图纸进行切割：抖动、光栅 G 代码、SVG 和 DXF 轮廓以及校准网格

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.1.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**诚实检查 - 当前真正能运行的内容：** 一个 Rust 库和一个命令行程序（`phoenix-laser`），有 609 个在计算机上运行的测试：用抖动把图像变成点，并写出逐行烧蚀、带动态功率和超程的激光 G 代码；读取 SVG 和 DXF 图纸，写出切割其轮廓的 G 代码（先内后外，考虑光束宽度、连接桥和多遍），并用线条雕刻其图形，各图层的设置放在一个文件里；还有功率对进给的测试网格。它有一个窗口（用 `--features gui` 构建），可打开图像或图纸、显示激光路径、修改设置并导出 G 代码，但窗口尚未在屏幕上查看过；也没有在机器上试过，没有用一批真实文件检查过。

---

> **安全状态：** 本程序不控制任何机器，也不是安全系统。它的输出在机器上运行之前必须由人检查；这里的任何内容都没有在真实硬件上运行过。


## 📋 速览

| | |
|---|---|
| 版本 | `0.1.2` |
| 成熟度 | 可用 |
| 测试 | 609 |
| 技术栈 | Rust |
| 真实硬件 | 未试过：这里的任何内容都没有在机器上运行过 |

**跳转到：** [概述](#-概述) · [架构](#-架构) · [快速开始](#-快速开始) · [测试](#-测试) · [结构](#-仓库结构) · [相关项目](#-相关项目)

## 🚦 项目状态

每一行是本仓库的一个部分，灯号表示它今天做到了哪一步。内容依据仓库能够展示的证据（测试、构建、运行），而不是计划。

🟢 已完成并验证 · 🟡 可用，有已知限制 · 🟠 部分完成或尚未验证 · 🔴 未完成或不可用

| 领域 | 状态 | 在这里的含义 |
|---|---|---|
| 核心 | 🟡 | 用抖动把图像变成点，并写出逐行烧蚀、带动态功率和超程的激光 G 代码；读取 SVG 和 DXF 图纸并按图层切割或雕刻（轮廓排序、光束宽度、连接桥、遍数、填充）；还有功率对进给的测试网格；已在计算机上用正常和损坏的输入测试，没有用一批真实文件检查。 |
| 命令行 | 🟡 | `phoenix-laser`（`engrave`、`calibrate`、`info`、`cut`、`vector`、`layers`）运行与库相同的函数并打印结果；每个错误都以消息和非零退出码返回。 |
| 测试 | 🟡 | 609 个测试通过（`cargo test`），包含正常情况、边界情况和损坏的输入。 |
| 安全 | 🟡 | 纯安全 Rust（`forbid(unsafe_code)`）：处理收到的文件时不会 panic，错误的值会返回带原因的错误；没有独立审查。 |
| 窗口 | 🟠 | 已编写（egui，`--features gui`）：打开或拖入图像或 SVG、DXF 图纸，查看激光路径（切割为单一颜色，空移变淡），编辑图层表，校准网格，缩放与平移，估算时间，导出；其逻辑已测试，但尚未构建，也没有在屏幕上查看过。 |
| 真实硬件 | 🔴 | 未试过：这里的任何内容都没有在机器上运行过。 |
| 文档 | 🟡 | 八种语言的 README、更新日志，以及由 `docs/` 绘制的两张图。 |

## 🎯 概述

* **图像**（`eh3d-image`）：PNG（灰度、彩色、透明，8 或 16 位）和 PGM（文本和二进制）被转为灰度，透明部分按白色处理，并限制尺寸，恶意文件无法要求数 GB 内存；`resize` 对像素取平均，使一个点与光斑一样大。
* **点阵**（`src/dither.rs`）：阈值，或 Floyd-Steinberg、Atkinson 误差扩散；测试检查灰度的深浅得以保持（中灰烧约一半的点），白色什么都不烧，黑色全部烧。
* **激光**（`src/raster.rs`、`src/calibrate.rs`）：逐行，先从左到右再从右到左，移动时 `M4` 加 `S0`，烧蚀处用所选功率，过扫为固定值或由加速度算出（v 的平方除以 2a），并提供每列一个功率、每行一个进给的方块网格，用来找到适合某种材料的参数。
* **每个 G 代码文件的文件头和数字**（`src/gcode_out.rs`）：文件以注释行开头，写明生成器及其版本、单位（mm，G21）、所用图片及其 SHA-256，以及工具（激光及其功率）；校准网格则注明没有来源。数字只有一种写法（最多三位小数，没有 `-0`），不是有限数的值会被拒绝，而不会被写成 `NaN` 或 `inf`。
* **`--dry-run`：** `engrave` 计算结果并统计（点数、行数、段数、烧蚀长度、时间），但不写入输出文件（此时可省略该文件）。
* **图纸**（`src/svg.rs`、`src/dxf.rs`、`src/flatten.rs`、`src/vector.rs`）：SVG 的安全子集（含全部命令的路径、图形、变换、分组、单位，以及对同一文件中元素的 `use`）和 DXF 的实体（直线、带圆弧的多段线、圆弧、圆、样条、单位）被转换为毫米图形；脚本、图像、对其他文件的 `use`、已声明的实体和一切未知内容会被跳过或拒绝，大小、深度、元素数（`use` 引入的也计入）和点数由设置限制，指回自身的 `use` 是错误，曲线在容差内变为直线段。
* **切割与雕刻**（`src/order.rs`、`src/offset.rs`、`src/cut.rs`）：图形按图层（DXF）或颜色（SVG）分组；每组按各自的设置切割（合并零散线条，先切位于其他轮廓内部的轮廓，空移最短，在正确一侧扣除光束宽度，保留连接桥，焦点逐遍推进的多遍切割）或雕刻（闭合图形按角度用线条填充，奇偶规则）。
* **设置文件**（`src/vconfig.rs`、`docs/vector-settings.md`）：所有调整过的值（读取器的限制、机器、各图层的设置）都在一个逐行检查的纯文本文件里；代码中没有这些值。
* **对焦测试**（`src/calibrate.rs`，设置文件中的 `[focus]`）：`phoenix-laser focus` 在不同的高度各烧一条短线，排成阶梯，最清晰的那条线显示光束的焦点位置；高度步进没有默认值（哪个方向向下取决于机器），功率服从功率上限，并且机头最后回到第一个高度。加上 `--powers 10,20,30`（最多 10 种功率）则改为烧刻对焦网格：同样的线条每种功率一列并排，从而可以读出在焦点高度烧出最细线条的功率；头每个高度只去一次并烧完该行的所有列，每种功率都遵守功率限制。
* **材料测试网格**（`src/calibrate.rs`，设置文件中的 `[calibrate]`）：`phoenix-laser calibrate --material 名称 --materials 文件` 围绕库中材料给出的值分配网格的功率和速度，保持在限制之内且不重复；`--labels` 用笔画字体在方块旁烧出每列的功率和每行的速度，之后可以直接在材料上读取网格；超过限制的材料会被拒绝，绝不会被降低。 使用材料时默认会烧出数字（`--no-labels` 可去掉），梯度的最小功率和最小速度是键 `min_power_percent` 和 `min_feed_mm_min`。
* **文字**（`src/text.rs`，设置文件中的 `[text]`）：`phoenix-laser text OUT.nc --text 文字` 用程序自带的笔画字体（大写字母、数字、空格和 `. , - + / : ! ? ( ) = _ '`；小写按大写烧刻，换行开始新的一行）烧刻名称、序列号或标记，无需字体文件。大写字母的高度、字母与行的间距、功率、速度和遍数都是设置；字体中没有的字符、超过 `max_chars` 个字符的文字、超过 `max_height_mm` 的高度、超出上限的功率或速度，或会超出台面的作业都会被拒绝而不会被裁剪，文字也绝不会写入程序的注释中。多行文字的各行可以左对齐、居中或右对齐（`--align`），文字可以按镜像烧刻，用于玻璃或亚克力的背面（`--mirror`），烧刻时间超过 `max_minutes` 的任务会被拒绝。
* **二维码**（`src/qr.rs`，设置文件中的 `[qr]`）：`phoenix-laser qr OUT.nc --text 数据` 把数据烧刻成程序自带的二维码（1 至 4 版，每边 21 至 33 个模块，字节模式，带四个级别的 Reed-Solomon 纠错和罚分最低的掩码），方法是用线条填充每个深色模块；该符号在所有版本和级别上都用独立的解码器读回验证过，测试也用自己的读取器读回。模块边长、线距、级别、功率、速度、道次和限制都是设置；超过 `max_bytes` 的数据、超过 `max_version` 的版本、大于 `max_size_mm` 的符号、超过 `max_lines` 次移动的填充、超出限制的功率或速度，或超出工作台的作业都会被拒绝，绝不裁剪或缩小，数据也绝不会写进程序的注释里。`--print` 以文本显示该符号。以下均默认关闭：`--invert` 改为烧刻浅色模块（用 `--border-modules` 可在周围加烧刻边框），`--mirror` 烧刻镜像，用于玻璃或亚克力背面，`--overscan-mm` 在关闭激光的情况下让机头在每条线前后助跑（若会超出台面则被拒绝），烧刻时间超过 `max_minutes` 的任务会被拒绝。
* **条形码**（`src/barcode.rs`，设置文件中的 `[barcode]`）：`phoenix-laser barcode OUT.nc --text 数据` 把数据烧刻成程序自带的 Code 128 条形码（空格到 `~` 的字符用 B 集，只含偶数个数字的文本用宽度减半的 C 集，并带校验符），方法是沿高度方向用线条填充每个条；测试用自己的读取器读回，但尚未用扫描器读取过。最窄条的宽度、高度、线距、功率、速度、道次和限制都是设置；不可打印字符的字节、超过 `max_chars` 的字符数、宽于 `max_width_mm` 的条形码、超过 `max_lines` 次移动的填充、耗时超过 `max_minutes` 的烧刻、超出限制的功率或速度，或超出工作台的作业都会被拒绝，绝不裁剪或缩小，数据也绝不会写进程序的注释里。`--mirror` 烧刻镜像，用于玻璃或亚克力的背面。
* **交叉填充**（`src/cut.rs`，图层中的 `hatch_cross`）：设为 `hatch_cross = on`（或在 `vector` 与 `trace` 中使用 `--cross`）的 `engrave` 图层，会在每一遍中用与第一组线成直角、间距相同的线再填充一次，线数上限对每一组都适用；图层列表给出的每平方毫米能量会翻倍，预览和程序检查把它当作任何其他程序来处理。
* **作业边框**（`src/frame.rs`，设置文件中的 `[frame]`）：`phoenix-laser frame` 根据尺寸或 G-code 程序烧蚀范围的外框，写出在激光关闭状态下描出作业矩形的程序，让人看到作业会落在哪里；微弱的指示功率每次都需要 `--confirm-laser-on`，上限 5 %（可由 `[frame]` 的 `max_pilot_percent` 修改），并服从功率上限，超出工作台的边框会被拒绝。
* **定位标记**（`src/frame.rs`，设置文件中的 `[marks]`）：`phoenix-laser marks OUT.nc --size W,H` 以较低功率在作业矩形的每个角（在边上，或在外侧 `margin_mm` 处）烧刻一个 L 形，加上 `--centre-cross` 还会在中心烧刻十字，使材料可以放回同一位置，并让第二次作业与第一次对齐。与在激光关闭时描框的边框不同，标记是真正烧刻的：功率和速度遵守机器的限制，长于矩形一边一半的臂，或会超出工作台的标记，会被拒绝，绝不裁剪。臂长、边距、功率、速度和道次都是设置。
* **预览**（`src/preview.rs`）：`phoenix-laser preview` 读取 G-code 程序（自己生成的或其他程序的），输出移动、烧蚀和空移的长度、范围、功率，以及计入拐角减速的时间；对看起来危险的情况发出警告（快速移动时激光开启、没有进给、功率超过满功率、工作超出工作台、没有结束），并把路径画成 SVG；测试会读回雕刻写出的程序并核对同样的长度。加上 `--laser-watts W`（激光满功率时的功率）后，还会给出程序烧刻的能量（每次移动的功率按 `--smax` 的比例乘以其烧刻时间）及其平均功率。
* **材料库**（`src/materials.rs`、`examples/materials.cfg`）：每种材料和加工方式适用的值（`[material 名称]`：功率、速度、遍数、焦距步进、光束宽度、填充线、点大小、抖动和色调曲线）只需写在一个文本文件里，用 `--material 名称 --materials 文件` 选用；命令行选项优先于它，功率上限对两者都有效。`phoenix-laser materials 文件` 列出材料库。
* **功率上限**（`src/safety.rs`、`[machine]` 中的 `max_power_percent`、`--max-power`）：功率超过上限的程序会被拒绝，错误信息指出图层或选项，绝不会悄悄调低；在文件或选项另有指定之前，上限是 100 %。
* **机器配置文件**（设置文件中的 `[profile NAME]`，`--profile NAME`）：拥有两台机器、或更换激光模块的人，可为每台保存一个配置（工作台行程、功率上限、速度上限、满功率数值和瓦数）；所选配置即为机器，`--max-speed` 与 `--max-power` 并列：超过上限的速度会被拒绝，绝不会被悄悄降低。
* **从图像提取轮廓**（`src/trace.rs`，设置文件中的 `[trace]`）：`phoenix-laser trace` 沿着徽标、剪影或模板的轮廓走，而不是逐点烧蚀；灰度按阈值划分，用亚像素精度的 marching squares 找到边界，简化到没有任何点离线超过 `simplify_mm`，并丢弃小斑点；结果是图纸的一个图层，使用相同的设置和相同的检查进行切割（或用线条填充），图像大小、边界片段、轮廓和点数都有上限。
* **窗口中的色调、材料和上限**（`src/window.rs`）：色调曲线的输入框（伽马、对比度、亮度、反相和自动色阶）使用与命令行相同的函数处理图像；材料库从文件加载（或由 `PHOENIX_LASER_MATERIALS` 指定），选择一种材料会填入功率、速度、点大小、抖动、色调以及图纸中每个图层的输入框；功率和速度上限同样是输入框，超过上限的值会被拒绝，绝不会被降低。初始值来自设置文件（`[tone]`、`[machine]`）。
* **窗口中的对焦测试**（`src/window.rs`）：一种带有 `[focus]` 部分输入框的模式（第一条线的高度、步进、线的数量和长度、间距、速度和功率）；测试使用与 `phoenix-laser focus` 相同的函数计算，其功率和速度遵守上限，没有默认步进，因为哪个方向向下取决于机器，并且测试会移动 Z 的警告保留在备注中。 窗口中有一个填写网格功率的输入框（最多 10 个，用逗号分隔），填写后窗口会烧刻 `focus --powers` 的对焦网格：每种功率烧一遍线条，每种功率一列；此时不使用单一功率的输入框，大于 0 且不超过 100 之外的功率、超过上限的功率或数量过多都会被拒绝。
* **窗口中的文字**（`src/window.rs`）：一个带有文字输入框和 `[text]` 部分各输入框的模式（大写字母的高度、最大字高、字间距和行间距、功率、速度、道次和最大字符数）；文字由与 `phoenix-laser text` 相同的函数从机器原点算出，所以字体中没有的字母、超出限制的文字、超出机器限制的功率或速度，或超出工作台的作业（当设置文件给出时）都会被拒绝，状态栏会说明原因；它以 `text.nc` 写出，带有命令行的文件头。行对齐、镜像开关和最长时间也可在窗口中设置。
* **窗口中的二维码**（`src/window.rs`）：一个带有数据输入框、纠错级别选择和 `[qr]` 部分各输入框的模式（模块边长、线距、功率、速度、道次、最大字节数、最大版本和边长，以及最大烧刻移动次数）；符号由与 `phoenix-laser qr` 相同的函数从机器原点算出，所以放不下的数据、超出限制的符号、超出机器限制的功率或速度，或超出工作台的作业（当设置文件给出时）都会被拒绝，状态栏会说明原因；它以 `qr.nc` 写出，带有命令行的文件头。反相和镜像开关，以及边框、助跑和最长时间的输入框也在窗口中提供。
* **窗口中的定位标记**（`src/window.rs`，设置文件中的 `[marks]`）：一个带有矩形尺寸、每个 L 的臂长、边距、功率、速度和道数输入框以及中心十字开关的模式（设置文件会填入 `[marks]` 的各项）；标记由与 `phoenix-laser marks` 相同的函数从机器原点算出，比半边更长的臂、超过上限的功率或速度、或超出工作台的标记都会被拒绝，状态栏会指出输入框或原因。它以 `marks.nc` 写出，带有通常的文件头。
* **窗口中的边框**（`src/window.rs`，设置文件中的 `[frame]`）：一种在激光关闭状态下框出当前打开的图像或图纸的模式；边距、圈数和速度都是输入框。大于 0 的指示功率会开启激光，因此窗口会先询问，在用户回答“是”之前不会计算或写出任何内容，而且每次都要确认（更改输入框、更改模式或写出文件都会收回回答），并受 `max_pilot_percent` 上限、功率上限和工作台范围的约束。
* **窗口中的预览**（`src/window.rs`）：窗口算出的每一份 G-code 都由与 `phoenix-laser preview` 相同的预览统计（移动次数、烧蚀与空行程长度、按输入框加速度计算拐角的时间，以及看起来危险的情况，例如作业超出设置文件的工作台范围），任何来源的程序（`.nc`、`.ngc`、`.gcode`、`.gc`、`.tap`）都可以打开或拖入窗口查看其路径和备注。它只作为文本读取，从不执行，也不会再次写出。 激光功率可以在输入框中填写（设置文件和机器配置会从 `laser_watts` 填入；0 表示未知）：预览随后还会给出程序烧刻的能量及其平均功率，与 `phoenix-laser preview --laser-watts` 相同，不是 0 到 1000 之间的数的值会被拒绝。
* **窗口中的轮廓**（`src/window.rs`，设置文件中的 `[trace]`）：一种模式，用与 `phoenix-laser trace` 相同的函数描摹已打开图片的轮廓，并将其切割，或用线条填充其形状，作为名为 `trace` 的图层；输入框有阈值、简化和最小面积，另有一个浅色为墨迹的开关；功率和速度就是激光输入框中的值并遵守限制，图片的限制（`max_pixels`、`max_contours`、`max_segments`）来自设置文件，因此过于精细的图片会报错，而不是让程序变慢。
* **窗口中的机器配置**（`src/window.rs`，设置文件中的 `[profile NAME]`）：侧边栏中列出文件里的机器；选中的那个就成为机器，与 `--profile` 的做法一致，因此满功率值以及功率和速度上限的输入框会取它的值，配置的工作台尺寸就是边框和预览所检查的尺寸。配置省略的项来自 `[machine]`，文件中没有的配置不会改变任何东西，上限仍会拒绝超过它的功率或速度。
* **窗口中按材料生成的网格**（`src/window.rs`，设置文件中的 `[calibrate]`）：校准模式会列出库中的材料；选中一种后，网格会按与 `phoenix-laser calibrate --material` 相同的梯度围绕其功率和速度展开（有步数、范围、最小功率和最小速度的输入框，并被限制在它们与上限之间），开关会按其输入框的高度在方块旁烧出数字。不选材料时，仍是两个列表的网格，与以前一样。
* **窗口中的程序检查**（`src/window.rs`，设置文件中的 `[verify]`）：为查看而打开的 G-code 程序也会由 `phoenix-laser verify` 按窗口的限制和设置文件中的台面进行检查，侧栏会说明它是否通过（附 SHA-256 的开头）或哪里有问题；一个输入框保存关闭激光时允许超出台面的余程（默认 0，最多 100 mm），在台面之外烧刻的程序无论输入框如何设置都不会通过。
* **程序检查**（`src/verify.rs`）：`phoenix-laser verify PROGRAM.nc` 将 G-code 程序（本程序或其他程序生成的）作为文本读取，从不执行，并给出该文件的 SHA-256、它的动作以及判定：只有在机器的功率和速度上限之下、位于工作台之内（工作台之外的加工算不通过；`--overscan-mm` 或 `[verify]` 中的 `overscan_mm` 允许机头在激光关闭时超出工作台这么远，就像其他程序的过扫描一样，而在工作台之外烧蚀始终不通过）、每次快速移动时激光关闭、每次烧蚀移动都有进给速度时才算通过。`--json` 输出报告，供其他程序与绑定到该哈希的授权一起保存（含机器配置、限制所依据的设置文件的 SHA-256 以及已执行检查的名称）；未通过时退出码为 1。
* **与已授权程序绑定的检查**（`src/verify.rs`）：`phoenix-laser verify PROGRAM.nc --expect-sha256 HASH` 在程序的 SHA-256 不是给定的 64 位十六进制数时判其不通过，并说明它实际的值，因此在查看之后被改动的程序不会被误运行；该失败也出现在 JSON 报告中。
* **有序抖动**（`src/dither.rs`）：`--dither bayer`（窗口中也可选）用 8 x 8 的拜耳矩阵，仅凭每个点的位置和灰度决定它是否烧蚀，因此图案规则，相同的灰度总是烧同样的点；测试检查矩阵的每个级别各出现一次，并且灰度的深浅得以保持。
* **色调曲线**（`src/tone.rs`）：生成点阵之前，可以把灰度拉伸到整个范围（`--auto-level`）、调整对比度和亮度、用伽马曲线弯曲或反相，只用一张 256 项的表；使用默认值时图像完全不变。
* **经过检查的 G 代码：**图纸的程序在交出之前逐行检查：快速移动时或没有速度时激光不会开启，`S` 不会超过满功率，不会移动到床面之外，结尾是 `M5`；文件绝不会写在它所来自的图纸或设置之上。

## 🧩 在 EH3D 中的角色

EH3D-PHOENIX-LASER 是 EH3D Fabrication Suite 的一个程序。它以库和命令行的形式工作，用 `--features gui` 构建时还是一个与套件其他程序外观一致的窗口。它读写简单的文件，因此可以单独使用，也可以用在其他程序之间。

## 🏛️ 架构

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-PHOENIX-LASER 的架构：谁在使用、包含什么、依赖什么" width="100%">
</p>


本仓库各部分如何配合：谁使用它、它包含什么、它依赖什么。图由 `docs/architecture.json` 绘制并与代码核对。

## 🖼️ 屏幕截图

这些图片由程序自身通过 `--shot` 截取（见快速开始）。

<p align="center">
  <img src="images/screens/window-vector.png" alt="切割图纸：激光在示例板上的路径，下方是图层表" width="100%">
</p>

<p align="center">
  <img src="images/screens/window-engrave.png" alt="雕刻图片：示例渐变的点按功率着色" width="100%">
</p>

## ✨ 功能

* **库加命令行：** `phoenix-laser` 的每个命令都是库中的函数，套件中的其他程序无需启动进程即可调用。
* **严格的命令行：** 写错的选项（`--powr`）或多余的词会产生指明它的错误，而不是被忽略；否则程序会用默认值生成输出。
* **损坏输入的测试：** 测试还会输入空文件、乱码、巨大数字和被截断的文件，任何情况都不允许 panic。
* **不猜测：** 没有意义的值是一个说明哪个值、为什么的错误，绝不是只是看起来正确的结果。
* **没有 unsafe 代码：** 库和程序中都有 `#![forbid(unsafe_code)]`。

## 🔒 安全与防护

| 威胁 | 防御 |
|---|---|
| 损坏、被截断或为空的文件 | 不会 panic：测试在每个位置截断有效输入并输入乱码 |
| 没有意义的数字（负数、非有限值、过大的数） | 以指出该值的错误拒绝；不会被舍入成看似合理的值 |
| 不安全的内存使用 | 带 `forbid(unsafe_code)` 的纯安全 Rust |
| 恶意图纸（脚本、已声明的实体、深层嵌套、数百万个点、巨大的数值） | 脚本、图像和链接绝不会被运行或获取；已声明的实体是错误；大小、深度、元素、图形、点和曲线的分段数都有限制；含有超出范围的点的图形会被丢弃 |
| 可能在没有速度时或超过满功率时打开激光的程序 | 每个程序在交出之前逐行检查，不合格的不会写出 |
| 超过机器或材料应承受的功率 | 功率上限（`max_power_percent`、`--max-power`）会拒绝超过它的程序，错误信息指出图层或选项；绝不会悄悄调低 |
| 损坏或巨大的材料库 | 文件的大小和材料数量有上限；超出范围的值、未知的键或重复的名称都是指出行号的错误；输出绝不会写在它上面 |
| 运行前先读取的外来 G-code 程序 | 预览只读取文本并限制大小，不执行也不下载任何东西，忽略荒谬的数字，并对快速移动时激光开启、没有进给、功率超过满功率、工作超出工作台和没有结束发出警告 |
| 仅为定位作业而点亮激光 | 边框在激光关闭时描出（不设功率，主轴从不开启）；指示功率即使设置文件要求也必须每次运行都加 `--confirm-laser-on`，在功率上限之内保持在上限值（除非 `max_pilot_percent` 另有设定，为 5 %）或更低，并在程序结束前关闭 |
| 对焦测试把机头朝 Z 的错误方向送出 | 高度步进没有默认值，在使用者给出之前测试会被拒绝；程序会说明它要移动 Z，范围受限（总共 50 mm），机头会回到第一个高度，功率服从功率上限 |
| 过长、过高、过强,或会烧到工作台之外的文字 | 只接受字体中有的字符(没有的字母会被拒绝,绝不跳过),字符数、高度、功率和速度保持在设置可以收窄的限制之内,超出工作台的作业会被拒绝而不是被裁剪,文字也绝不会写进程序的注释里 |
| 放不进二维码的数据、过大、过密或过强的符号，或会烧到工作台之外的符号 | 版本、字节数、尺寸、烧刻移动次数、功率和速度保持在设置可以收窄的限制之内，超出的会被拒绝而不是被裁剪或缩小；数据也绝不会写进程序的注释里 |
| 选在图纸或所依据的设置之上的输出文件 | 在窗口和命令中都会被拒绝 |

**没有防御的内容：** 无错误产生的结果不等于经过验证的结果。这里没有任何内容了解机器的极限，程序也没有经过独立审查。请私下报告漏洞（见 [SECURITY.md](SECURITY.md)）。

## 🔌 接口

库为 `eh3d_phoenix_laser`（`use eh3d_phoenix_laser::...`），命令行为 `phoenix-laser`。命令如下；每个命令把结果打印到标准输出，把错误以非零退出码报告到标准错误。没有网络访问。

```text
phoenix-laser engrave IMAGE OUT.nc --width-mm 50 [--dot-mm 0.1] [--feed 3000] [--power 50] [--smax 1000]
              [--dither fs|atkinson|threshold|bayer] [--threshold 128] [--overscan MM|auto] [--accel 1000] [--origin X,Y] [--dry-run]
              [--gamma 1] [--contrast 0] [--brightness 0] [--invert] [--auto-level] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
              [--material NAME --materials FILE]
phoenix-laser calibrate OUT.nc --powers 20,40,60,80 --feeds 1000,2000,3000 [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser calibrate OUT.nc --material NAME --materials FILE [--steps 5] [--spread 40] [--labels|--no-labels] [--label-mm 3] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser text OUT.nc --text TEXT [--height-mm 10] [--spacing 25] [--line-spacing 50] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--align left|center|right] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser qr OUT.nc --text DATA [--ecc L|M|Q|H] [--module-mm 1] [--line-mm 0.1] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--invert] [--border-modules 0] [--overscan-mm 0] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--print] [--config FILE] [--profile NAME]
phoenix-laser barcode OUT.nc --text DATA [--module-mm 0.4] [--height-mm 10] [--line-mm 0.1] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser marks OUT.nc --size W,H [--origin X,Y] [--arm-mm 5] [--margin-mm 0] [--power 10] [--feed 1000] [--passes 1] [--centre-cross|--no-centre-cross] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser info IMAGE
phoenix-laser cut DRAWING OUT.nc [--config FILE] [--power P] [--speed MM/MIN] [--passes N] [--focus-step MM] [--kerf MM] [--kerf-side none|auto|inside|outside]
              [--tabs N] [--tab-width MM] [--smax N] [--overscan MM] [--origin X,Y] [--bed X,Y] [--max-power P] [--max-speed MM/MIN] [--profile NAME] [--material NAME --materials FILE] [--dpi N] [--tolerance MM] [--dry-run]
phoenix-laser vector DRAWING OUT.nc [the options of cut] [--angle DEG] [--spacing MM] [--cross|--no-cross]
phoenix-laser layers DRAWING [--config FILE] [--dpi N]
phoenix-laser trace IMAGE OUT.nc --width-mm W [--threshold 128] [--invert] [--simplify MM] [--min-area MM2] [--engrave] [the options of cut]
phoenix-laser materials FILE
phoenix-laser focus OUT.nc --z-step MM [--z-start MM] [--steps 9] [--length 10] [--spacing 2] [--feed 1000] [--power 30 | --powers 10,20,30] [--smax 1000] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser frame OUT.nc --size W,H | --gcode PROGRAM.nc [--origin X,Y] [--margin MM] [--feed 3000] [--laps 1] [--bed X,Y] [--smax N]
              [--pilot P --confirm-laser-on] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser verify PROGRAM.nc [--config FILE] [--profile NAME] [--smax N] [--max-power P] [--max-speed MM/MIN] [--bed X,Y] [--overscan-mm MM] [--accel 1000] [--expect-sha256 HASH] [--json]
phoenix-laser preview PROGRAM.nc [OUT.svg] [--accel 1000] [--rapid 6000] [--smax N] [--laser-watts W] [--bed X,Y] [--width-px 900] [--config FILE] [--profile NAME]
phoenix-laser [gui] [FILE]                       the window (a build with --features gui); with no arguments it opens
```

`DRAWING` 是 SVG 或 DXF 文件。`cut` 沿轮廓切割（先切内部的）；`vector` 用线条填充闭合图形并沿开放图形走线；两者都从 `--config FILE` 读取每个图层或颜色的设置（见 [docs/vector-settings.md](docs/vector-settings.md)），命令行选项会对所有图层生效。`layers` 列出图纸的各个图层以及对每个图层将执行的处理。

## ⚙️ 配置

图像的雕刻通过命令行参数设置（见“接口”），每个默认值都写在命令的帮助里（`phoenix-laser help`）。图纸的切割和雕刻通过设置文件设置（`--config FILE`，或窗口中的“加载设置”）：读取器的限制、文件的读取方式、切割顺序、机器，以及每个图层或颜色的模式（切割、雕刻或跳过）、功率、速度、遍数、焦点步进、光束宽度及其位置、连接桥和填充。每个键、它的默认值和范围都在 [docs/vector-settings.md](docs/vector-settings.md) 中，示例见 `examples/plate.cfg`；未知的键或超出范围的值会产生指明所在行的错误。图像读取器的固定限制以具名常量的形式放在代码里，并有测试覆盖。

## 🚀 快速开始

需要稳定版 Rust 工具链（1.88 或更新）。需要把 EH3D-COMMON 仓库克隆到本仓库旁边：图像读取器通过路径从中引入。

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/plate.svg   # a picture of the window
```

要在不手动打开窗口的情况下截取窗口图像，请使用 `--shot FILE.png` 运行（还可用 `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`）：它按该尺寸打开，保存 PNG，不保留任何设置并自动关闭（写入文件成功时退出码为 0，否则为 1，参数错误时为 2）。

## 🧪 测试

测试套件有 609 个测试（`cargo test`，CI 在 Windows 和 Linux 上运行）。它们检查正常情况、边界情况和损坏的输入：空文件、乱码和被截断的文件绝不会 panic。CI 还运行 `cargo fmt --check`、把警告当作错误的 `cargo clippy` 以及 `python -I tools/ci_validate.py`。

## 🏗️ 构建与发布

版本号位于 `Cargo.toml` 和 `eh3d.project.json`；每个版本在 [CHANGELOG.md](CHANGELOG.md) 中都有条目。`build.bat` / `build.sh` 构建程序，`test.bat` / `test.sh` 运行测试，`run.bat` / `run.sh` 用示例运行。版本发布在 GitHub 上。

**下载：** 本仓库的每个版本都会发布适用于各系统的现成程序：Windows（64 位和 32 位）、Linux（64 位）以及 macOS（Apple 芯片和 Intel）。每个软件包（`phoenix-laser-<版本>-<系统>.zip` 或 `.tar.gz`）包含程序、许可证、本 README、更新日志、图标和示例文件，并附带其 `.sha256` 文件；该版本还在 `SHA256SUMS` 和 `build-manifest.json` 中列出所有文件。目前尚未发布任何版本。

**已签名且可校验：** 版本中的软件包使用 EH3D 的发布密钥签名，每个软件包旁边都会发布其 `.sig` 文件（OpenSSH 签名，命名空间 `eh3d-release`）。运行下载的文件之前请先校验：比较其 SHA-256（`sha256sum -c phoenix-laser-<版本>-<系统>.tar.gz.sha256`，Windows 上用 `Get-FileHash`），并用 `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <软件包>.sig < <软件包>` 验证签名，其中 `allowed_signers` 含有一行 `eh3d-release <密钥类型> <EH3D 发布公钥>`。

## 📂 仓库结构

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-PHOENIX-LASER 的仓库结构" width="100%">
</p>

<details>
<summary>目录树的文本版</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   库：每个命令所使用的部分
│   ├── main.rs                                  命令行：engrave、calibrate、info、cut、vector、trace、verify 和 layers
│   ├── calibrate.rs                             逻辑的一部分
│   ├── cli_args.rs                              命令行参数：每个小工具中的同一个文件
│   ├── dither.rs                                逻辑的一部分
│   ├── materials.rs                             材料库：功率、速度、遍数、点阵和色调
│   ├── preview.rs                               G-code 预览：移动、时间、风险和路径 SVG
│   ├── tone.rs                                  图像的色调曲线：生成点阵前的色阶、对比度、亮度、伽马和反相
│   ├── trace.rs                                 图像的轮廓：阈值、marching squares 和简化的环，带有限制
│   ├── text.rs                                  用自带笔画字体烧刻的文字，受限制约束
│   ├── qr.rs                                    自带的二维码，1 至 4 版并带纠错，通过填充模块烧刻
│   ├── barcode.rs                               自带的 Code 128 条形码，通过用线条填充条来烧刻
│   ├── gcode_out.rs                             G 代码数字的唯一写法和标准文件头
│   ├── raster.rs                                逻辑的一部分
│   ├── safety.rs                                功率上限，以及展示工作范围时保持激光关闭的规则
│   ├── shot.rs                                  窗口的截图模式（--shot）：无需屏幕即可读取和检查其参数
│   ├── vector.rs                                读入的图纸：毫米图形、图层、限制和基础几何
│   ├── verify.rs                                程序检查：其 SHA-256、动作以及对照机器上限的判定
│   ├── flatten.rs                               在容差内将曲线分段为直线（贝塞尔、圆弧）
│   ├── frame.rs                                 作业边框（在激光关闭时描出的矩形）及其四角的定位标记
│   ├── svg.rs                                   SVG 读取器：安全子集，不含脚本、图像和实体
│   ├── dxf.rs                                   DXF 读取器：直线、多段线、圆弧、圆和样条，含单位
│   ├── energy.rs                                由光功率算出每毫米或每平方毫米一次走刀的能量
│   ├── offset.rs                                将轮廓按光束宽度侧移，逐边校验
│   ├── order.rs                                 切割顺序：合并散线、先内后外、空移最短
│   ├── cut.rs                                   从图纸到 G-code：切割、连接桥、填充、遍数和安全检查
│   ├── vconfig.rs                               矢量作业的设置文件：限制、机器和各图层
│   ├── browse.rs                                打开文件窗口的文件列表
│   ├── gui.rs                                   窗口（egui，gui 特性）：绘制状态并传递用户的操作
│   ├── lang.rs                                  窗口的文字（八种语言）
│   ├── viewport.rs                              图像的缩放和平移（纯数值）
│   └── window.rs                                无屏幕的窗口状态：图像或图纸、设置、图层、遍数和导出
├── tests/
│   ├── fuzz.rs                                  固定种子的测试：损坏的文件、截断和异常数字
│   ├── dry_run.rs                               --dry-run 测试：计数相同且不写入任何内容
│   ├── laser.rs                                 库和命令行的测试
│   ├── laser_tools.rs                           通过命令行测试雕刻周边的工具
│   ├── readme.rs                                运行 README 和 run 脚本所承诺内容的测试
│   ├── strict.rs                                命令行测试：它不认识的选项和词
│   ├── vector.rs                                图纸切割测试：命令、示例、恶意文件和 G-code 检查
│   └── window_cli.rs                            测试没有 gui 特性的构建仍然只是命令行
├── examples/
│   ├── gradient.pgm                             试用的示例
│   ├── plate.svg                                带孔和待雕刻面板的板（SVG）
│   ├── plate.dxf                                同一块板的 DXF
│   ├── plate.cfg                                各图层的设置：功率、速度、遍数、光束宽度、连接桥和填充
│   └── materials.cfg                            材料库：桦木和板岩，切割、雕刻和照片
├── docs/
│   ├── architecture.json                        架构图数据
│   ├── structure.json                           结构图数据
│   ├── vector-settings.md                       矢量作业设置文件，逐项说明
│   └── materials.md                             材料库，逐项说明
├── tools/                                       CI 和 README 检查及绘图工具
├── images/                                      横幅图、README 图片及窗口截图
├── .github/                                     CI 与发布工作流、Issue 和 PR 模板
├── eh3d.project.json                            清单：版本、角色、成熟度和构建命令
├── Cargo.toml Cargo.lock                        工作区及其锁定的依赖
├── rust-toolchain.toml                          固定的 Rust 工具链
├── CHANGELOG.md                                 每个版本的变更
├── SECURITY.md SUPPORT.md                       如何报告漏洞及在哪里提问
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   参与贡献、行为准则和许可证
```

</details>

## 🗺️ 路线图与已知限制

**目前已知的限制**

* 窗口尚未在屏幕上查看过。
* 尚未在机器上或用一批真实文件试过，也没有经过独立审查。
* 图纸只按安全子集读取（不含对其他文件的 `use`、文字、渐变和带选择器的样式；DXF 中不含椭圆、填充和块）；比光束更窄的轮廓不会被切割，程序会说明；激光是使用 `M4` 的 GRBL 控制器的激光，没有在真实的激光器或材料上试过任何东西。

**下一步**

* 在烧蚀前（设置 `laser_watts` 后，`layers` 已会打印一次走刀的能量）于板上显示功率密度，并在切割的拐角处保持功率恒定。
* 在屏幕上查看窗口并完善它。
* 用真实图纸和套件中的其他程序试用。

## 🌍 语言

README 和文本提供英语、西班牙语、法语、意大利语、德语、汉语、日语和欧洲葡萄牙语版本。

## 🔗 相关项目

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON)：共享 crate，其中包括图像读取器
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): 锡膏钢网
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): G 代码到达机器之前的检查
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): CAM 程序共用的刀具库
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): 机器控制程序
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): 查找并启动套件程序的中心

## 📚 文档

* [更新日志](CHANGELOG.md)
* [架构与结构数据](docs/architecture.json)：绘制两张图所用的数据。
* [矢量作业的设置](docs/vector-settings.md)：设置文件的每个键及其默认值和范围。
* [材料库](docs/materials.md)：材料的每个键以及材料如何应用

## 🤝 参与贡献与社区

* 提交拉取请求前请运行 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test` 和 `python -I tools/ci_validate.py`：必须全部通过。见 [CONTRIBUTING.md](CONTRIBUTING.md) 和[行为准则](CODE_OF_CONDUCT.md)。
* README 有八种语言：修改文字要在八个文件中同时进行，两张图由 `tools/make_architecture.py` 和 `tools/make_structure.py` 根据 `docs/architecture.json` 和 `docs/structure.json` 绘制。
* 请私下报告漏洞：[SECURITY.md](SECURITY.md)。问题、想法和报告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 许可证

软件：GPL-3.0-or-later。见 [LICENSE](LICENSE)。套件的硬件设计使用 CERN OHL-S。
