use std::collections::HashMap;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use std::sync::{Arc, Mutex};


#[derive(Clone)]
pub struct TtsEngine {
    session: Arc<Mutex<Session>>,
    lexicon: Arc<HashMap<String, Vec<i64>>>, // 🐙 新增：常驻内存的全量词典
}

impl TtsEngine {
    // 1. 返回值不再是 Box<dyn Error...>，而是极其简单的 Result<Self, String>
    pub fn new(model_path: &str, lexicon_path: &str) -> Result<Self, String> {
        let is_init = ort::init()
            .with_name("OpenTako_Core")
            .commit();

        println!("🧠 正在加载神经网络模型: {} @@ {}", model_path, is_init);

        // 2. 在每个可能报错的链式调用后面加上 .map_err(|e| e.to_string())?
        // 这一步的作用是把包含裸指针的 ort::Error 销毁，只提取它的文字错误描述
        let session = Session::builder()
            .map_err(|e| e.to_string())?
            .with_optimization_level(GraphOptimizationLevel::Level1)
            .map_err(|e| e.to_string())?
            .with_intra_threads(4)
            .map_err(|e| e.to_string())?
            .commit_from_file(model_path)
            .map_err(|e| format!("🚨 模型加载失败 (路径不对或模型文件损坏): {}", e))?;

        println!("✅ 模型常驻内存完成 (Hot 状态)");

        // 🐙 新增：加载并解析 JSON 词典文件
        println!("📖 正在加载词典文件: {}", lexicon_path);
        let lexicon_str = std::fs::read_to_string(lexicon_path)
            .map_err(|e| format!("🚨 无法读取词典文件 (请检查路径): {}", e))?;

        // 使用 serde_json 将字符串直接反序列化为 HashMap
        let lexicon: HashMap<String, Vec<i64>> = serde_json::from_str(&lexicon_str)
            .map_err(|e| format!("🚨 词典 JSON 格式错误: {}", e))?;

        println!("✅ 引擎初始化完成! (已加载 {} 个词汇)", lexicon.len());

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            lexicon: Arc::new(lexicon), // 存入引擎
        })
    }


    // 3. 返回值改为 Result<Vec<u8>, String>
    pub fn generate_audio(&self, text: &str) -> Result<Vec<u8>, String> {
        let mut raw_phoneme_ids: Vec<i64> = Vec::new();
        raw_phoneme_ids.push(1); // 句子起始符 ^

        // 🐙 新增：带标点感知的智能分词器
        // 按照空格切分，但我们会检查单词末尾是否带有标点
        for word in text.to_lowercase().split_whitespace() {
            // 提取纯字母部分用于查字典 (比如把 "world," 变成 "world")
            let clean_word: String = word.chars().filter(|c| c.is_alphabetic()).collect();

            if clean_word.is_empty() { continue; }

            // 查表：O(1) 极速哈希碰撞
            if let Some(ids) = self.lexicon.get(&clean_word) {
                raw_phoneme_ids.extend_from_slice(ids);
            } else {
                // OOV (Out Of Vocabulary) 兜底策略
                // 真实场景下可以回退到按字母逐个发音，这里先默认发出 "Hmm" (20, 25) 的声音
                println!("🧠 使用自然拼读推测发音: {}", clean_word);
                let guessed_ids = guess_word_phonemes(&clean_word);

                if guessed_ids.is_empty() {
                    // 如果连拼读都失败了（比如全是数字或符号），再回退到 "hmm"
                    raw_phoneme_ids.extend_from_slice(&[20, 25]);
                } else {
                    raw_phoneme_ids.extend(guessed_ids);
                }
            }

            // --- 标点与停顿处理 ---
            // Piper 极度依赖标点来控制语气的起伏和断句
            if word.ends_with(',') {
                raw_phoneme_ids.push(8);  // 逗号的 ID 是 8 (短暂停顿)
            } else if word.ends_with('.') || word.ends_with('!') {
                raw_phoneme_ids.push(10); // 句号的 ID 是 10 (长停顿/降调)
            } else if word.ends_with('?') {
                raw_phoneme_ids.push(13); // 问号的 ID 是 13 (升调)
            } else {
                raw_phoneme_ids.push(3);  // 正常的单词间空格 ID 是 3
            }
        }

        raw_phoneme_ids.push(2); // 句子结束符 $

        // 🚀 保持之前的 Intersperse (交错占位) 逻辑不变！
        let mut interspersed_ids: Vec<i64> = Vec::with_capacity(raw_phoneme_ids.len() * 2 + 1);
        interspersed_ids.push(0);
        for &id in &raw_phoneme_ids {
            interspersed_ids.push(id);
            interspersed_ids.push(0);
        }

        let seq_len = interspersed_ids.len();
        if seq_len == 0 { return Ok(Vec::new()); }

        // --- 准备张量与推理 (保持完全不变) ---
        let input_tensor = ort::value::Tensor::from_array(([1, seq_len], interspersed_ids))
            .map_err(|e| e.to_string())?;
        let input_lengths_tensor = ort::value::Tensor::from_array(([1], vec![seq_len as i64]))
            .map_err(|e| e.to_string())?;
        let scales_tensor = ort::value::Tensor::from_array(([3], vec![0.667_f32, 1.0, 0.8]))
            .map_err(|e| e.to_string())?;

        let mut session_guard = self.session.lock().unwrap();
        let outputs = session_guard.run(ort::inputs![
            "input" => input_tensor,
            "input_lengths" => input_lengths_tensor,
            "scales" => scales_tensor,
        ]).map_err(|e| e.to_string())?;

        let (_shape, audio_slice) = outputs["output"].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;

        let mut byte_stream = Vec::with_capacity(audio_slice.len() * 4);
        for &sample in audio_slice {
            byte_stream.extend_from_slice(&sample.to_le_bytes());
        }

        Ok(byte_stream)
    }
}

// 🐙 纯 Rust 智能自然拼读引擎 V2 (支持 Magic 'e' 和 开闭音节)
fn guess_word_phonemes(word: &str) -> Vec<i64> {
    let chars: Vec<char> = word.chars().collect();
    let mut phonemes = Vec::new();
    let mut i = 0;

    let is_vowel = |c: char| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y');

    while i < chars.len() {
        let c = chars[i];

        // 0. 词尾静音 'e' (Silent 'e' 规则)
        // 如果 'e' 是单词的最后一个字母，且前面还有其他字母，直接跳过不发音
        if c == 'e' && i == chars.len() - 1 && i > 1 {
            break;
        }

        // 1. 双字母组合 (Digraphs)
        if i + 1 < chars.len() {
            let digraph = format!("{}{}", chars[i], chars[i + 1]);
            let mut matched = true;
            match digraph.as_str() {
                "sh" => phonemes.push(96),                     // ʃ
                "ch" => phonemes.extend_from_slice(&[32, 96]), // tʃ
                "th" => phonemes.push(126),                    // θ (清辅音)
                "ph" => phonemes.push(19),                     // f
                "ee" | "ea" => phonemes.push(21),              // i (长音e)
                "oo" => phonemes.push(33),                     // u
                "oa" => phonemes.extend_from_slice(&[27, 100]), // oʊ
                "ai" | "ay" => phonemes.extend_from_slice(&[18, 74]), // eɪ
                "ar" => phonemes.extend_from_slice(&[51, 30]), // ɑ r
                "er" | "ir" | "ur" => phonemes.push(60),       // ɚ (美式卷舌)
                "or" => phonemes.extend_from_slice(&[54, 30]), // ɔ r
                "ou" | "ow" => phonemes.extend_from_slice(&[14, 100]), // aʊ
                "qu" => phonemes.extend_from_slice(&[23, 35]), // k w
                "ck" => phonemes.push(23),                     // k
                "ng" => phonemes.push(44),                     // ŋ
                _ => matched = false,
            }
            if matched {
                i += 2;
                continue;
            }
        }

        // 2. 元音智能处理 (区分长短音)
        if is_vowel(c) {
            let mut is_long = false;

            // 规则 A: Magic 'e' (元-辅-e 结构) -> 触发长音 (例如 vibe, make)
            if i + 2 < chars.len() && !is_vowel(chars[i + 1]) && chars[i + 2] == 'e' && i + 3 == chars.len() {
                is_long = true;
            }
            // 规则 B: 开音节 (V-C-V 结构) -> 前一个元音发长音 (例如 O-pen, Ta-ko)
            else if i + 2 < chars.len() && !is_vowel(chars[i + 1]) && is_vowel(chars[i + 2]) {
                is_long = true;
            }
            // 规则 C: 处于单词绝对末尾的元音 -> 发长音 (例如 O, fly, Tako)
            else if i == chars.len() - 1 {
                is_long = true;
            }

            if is_long {
                // 长音 (字母本身的读音)
                match c {
                    'a' => phonemes.extend_from_slice(&[18, 74]), // /eɪ/
                    'e' => phonemes.push(21),                     // /i/
                    'i' | 'y' => phonemes.extend_from_slice(&[51, 74]), // /aɪ/
                    'o' => phonemes.extend_from_slice(&[27, 100]), // /oʊ/
                    'u' => phonemes.extend_from_slice(&[37, 33]), // /ju/
                    _ => {}
                }
            } else {
                // 短音 (闭音节)
                match c {
                    'a' => phonemes.push(39),  // /æ/ (如 apple)
                    'e' => phonemes.push(61),  // /ɛ/ (如 bed)
                    'i' | 'y' => phonemes.push(74),  // /ɪ/ (如 sit)
                    'o' => phonemes.push(51),  // /ɑ/ (如 hot)
                    'u' => phonemes.push(102), // /ʌ/ (如 up)
                    _ => {}
                }
            }
            i += 1;
            continue;
        }

        // 3. 辅音智能处理 (软硬音)
        match c {
            'b' => phonemes.push(15),
            'c' => {
                // 软 C 规则: 遇到 e, i, y 发 /s/ (如 center)，否则发 /k/ (如 cat)
                if i + 1 < chars.len() && matches!(chars[i+1], 'e' | 'i' | 'y') {
                    phonemes.push(31); // s
                } else {
                    phonemes.push(23); // k
                }
            }
            'd' => phonemes.push(17),
            'f' => phonemes.push(19),
            'g' => {
                // 软 G 规则: 遇到 e, i, y 发 /dʒ/ (如 magic)，否则发 /g/ (如 go)
                if i + 1 < chars.len() && matches!(chars[i+1], 'e' | 'i' | 'y') {
                    phonemes.extend_from_slice(&[17, 108]); // dʒ
                } else {
                    phonemes.push(66); // ɡ
                }
            }
            'h' => phonemes.push(20),
            'j' => phonemes.extend_from_slice(&[17, 108]),
            'k' => phonemes.push(23),
            'l' => phonemes.push(24),
            'm' => phonemes.push(25),
            'n' => phonemes.push(26),
            'p' => phonemes.push(28),
            'r' => phonemes.push(30),
            's' => phonemes.push(31),
            't' => {
                // 快速识别 -tion 后缀
                if i + 3 < chars.len() && chars[i + 1] == 'i' && chars[i + 2] == 'o' && chars[i + 3] == 'n' {
                    phonemes.push(96); // ʃ
                } else {
                    phonemes.push(32); // t
                }
            }
            'v' => phonemes.push(34),
            'w' => phonemes.push(35),
            'x' => phonemes.extend_from_slice(&[23, 31]), // ks
            'z' => phonemes.push(38),
            _ => {}
        }
        i += 1;
    }

    phonemes
}

// 🐙 文本清洗与智能切片器 (Text Normalization & Chunking)
pub fn normalize_and_chunk(raw_text: &str) -> Vec<String> {
    // 1. 基础文本清洗：替换各种奇葩的引号和连字符，转为标准 ASCII 标点
    let cleaned = raw_text
        .replace(['“', '”', '「', '」'], "\"")
        .replace(['‘', '’'], "'")
        .replace(['—', '–'], "-")
        .replace(['（', '）'], "") // 简单起见，过滤掉括号
        .replace('\n', " "); // 把换行符转为空格，靠标点来断句

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    // 2. 智能切片：遇到终端标点符号就切断
    for c in cleaned.chars() {
        current_chunk.push(c);

        // 当遇到句号、感叹号、问号、或者分号时，认为是一句完整的话
        if c == '.' || c == '!' || c == '?' || c == ';' {
            let chunk = current_chunk.trim().to_string();
            // 过滤掉太短的无效切片（比如连续的 "..."）
            if chunk.len() > 1 {
                chunks.push(chunk);
            }
            current_chunk.clear();
        }
    }

    // 3. 收尾：如果最后一段没有标点符号，也要把它加进去
    if !current_chunk.trim().is_empty() {
        // 强制加上一个句号，让模型在最后能够正常降调收尾
        let final_chunk = format!("{}.", current_chunk.trim());
        chunks.push(final_chunk);
    }

    chunks
}