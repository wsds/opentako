<div align="center">
  <h1>🐙 OpenTako (开源章鱼)</h1>
  <p><strong>专为端侧小模型打造的零延迟、零算力成本多模态 AI 流水线引擎</strong></p>

  <a href="https://github.com/wsds/OpenTako/blob/main/LICENSE"><img alt="License: AGPL v3" src="https://img.shields.io/badge/License-AGPL_v3-blue.svg"></a>
  <a href="https://rust-lang.org"><img alt="Written in Rust" src="https://img.shields.io/badge/Language-Rust-orange.svg"></a>
  <img alt="Platform" src="https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey">
  <img alt="Status" src="https://img.shields.io/badge/Status-Beta-success">

  <p>
    <a href="https://opentako.ai.寒武纪.com/">访问官网</a> •
    <a href="#quick-start">快速开始</a> •
    <a href="#showcase">真实应用案例</a> •
    <a href="#architecture">架构解析</a>
  </p>
</div>

---

## 💡 为什么需要 OpenTako？

**“杀鸡焉用牛刀？大模型是大脑，但应用高频交互只需要‘肌肉记忆’。”**

开发者目前在端侧接入 AI 面临两难：
1. 依赖云端 API（如 OpenAI）：高昂的边际成本（越用越亏）、不可忍受的网络延迟、绝对的隐私禁区。
2. 自己打包 Python/PyTorch 环境：几 GB 的庞大体积、恐怖的内存占用、各种显卡驱动崩溃。

**OpenTako 是端侧 AI 应用的“路由器”。** 我们战略性放弃百亿参数大语言模型（LLM）的红海，专攻 **30M~1.5B 级别** 的开源工程小模型（TTS、OCR、NMT、VAD）。通过极其底层的 Rust 内存级流水线编排，榨干用户设备闲置算力。

## ✨ 核心特性

* ⚡️ **挑战物理极限的零延迟：** 完全离线本地运行，告别云端 API 动辄几百毫秒的网络往返时间。
* 🧠 **内存级零拷贝流水线 (Zero-Copy Pipeline)：** 首创将多个跨模态小模型（`OCR提取 -> 翻译 -> 语音朗读`）在 Rust 物理内存指针间直接流转，彻底消除进程间序列化耗时。
* 🔌 **无缝平替 OpenAI API (Drop-in Replacement)：** 引擎内置轻量 HTTP 服务器，完全兼容 OpenAI API 标准。**只需改一行 `BASE_URL`，即可将云端算力零摩擦切换至本地。**
* 🪶 **极速轻量与动态显存：** 纯 Rust + ONNX Runtime 构建，无 Python 依赖。微型模型（如 VAD）常驻内存 0 毫秒启动，中型模型按需驻留，老旧轻薄本也能流畅起飞。

---

## 🚀 快速开始 (Quick Start)

### 1. 启动引擎
下载预编译的二进制文件（[Release 页面下载](#)），无需配置任何环境，双击直接运行：

```bash
# 引擎将自动在后台启动，并监听本地端口
./opentako
````

*(引擎首次调用某个模型时，会自动进行极速断点续传下载，真正开箱即用)*

### 2\. 像调用 OpenAI 一样调用本地 TTS

无需学习任何新协议，直接用你熟悉的 HTTP 客户端或 OpenAI SDK：

```javascript
// Node.js 示例
import OpenAI from "openai";

const openai = new OpenAI({
  baseURL: "http://localhost:8080/v1", // 指向 OpenTako 本地引擎
  apiKey: "sk-opentako-local",         // 密钥随便填
});

async function main() {
  const mp3 = await openai.audio.speech.create({
    model: "tako-tts-en-v1",           // 调用本地离线 TTS 模型
    voice: "alloy",
    input: "Hello from OpenTako! Zero latency, zero cost.",
  });
  // 此时音频流已通过本地极速生成
}
main();
```

-----

## 🛠 生产级展示案例 (Showcase)

**Talk is cheap, we have the product.** OpenTako 绝不是一个停留在实验室里的玩具。它目前正在驱动高净值教育 SaaS 应用 —— **DeepReader (AI 沉浸式外刊阅读器)**。

  * **痛点：** 外刊精读应用中，用户每天海量的“点按查词”与“段落朗读”，如果走云端 API 会产生巨额账单。
  * **解决方案：** DeepReader 接入 OpenTako 后，所有高频 TTS 与翻译交互全部由本地引擎接管，**算力边际成本直接降为 0**，实现了惊人的业务毛利率，并在低配轻薄本上实现了如丝般顺滑的纯离线体验。
-----

## 🏗 架构演进路线 (Roadmap)

  - [x] 基于 Rust + ONNX 的核心推理底座
  - [x] WebSocket 流式双向通信支持
  - [x] 成功闭环“1号试飞员”应用 (DeepReader)
  - [ ] 兼容 OpenAI RESTful API 标准格式
  - [ ] 内置多线程、断点续传的模型包自动下载器 (Hub)
  - [ ] 支持基于 JSON/YAML 的动态流水线编排配置
  - [ ] 提供主流前端框架 (React/Vue) 的一键接入 SDK

-----

## 📄 商业与授权协议 (License)

OpenTako 致力于构建健康的开发者生态，采用 **双重授权模式 (Dual Licensing)**：

1.  **开源社区版：** 基于 [AGPL v3 License](https://www.google.com/search?q=./LICENSE) 授权。欢迎极客和独立开发者免费使用。
2.  **商业闭源授权 (Commercial License)：** 如果您需要将 OpenTako 嵌入到闭源商业软件、SaaS 桌面端或智能硬件（学习机/软路由/NAS 等）中进行分发，并希望保留代码的闭源状态，请与我们联系获取商业授权许可。

📫 **联系我们探讨商业合作：** dev@mail.寒武纪.com
-----

<div align="center">
    <b>如果 OpenTako 帮你省下了云端 API 的账单，请给我们点亮一个 ⭐️ Star！</b>
</div>