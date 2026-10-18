package com.kshana.app.ui.screens

import uniffi.kshana_ffi.TopicRecord

/** What a question in the person's own words was understood as. */
internal data class Understood(val life: String, val topic: TopicRecord?, val question: String)

/**
 * Turns a question typed or spoken in English, Malayalam or Hindi into the app's
 * terms: which part of life, which matter, and which kind of question. Simple word
 * matching, done on the phone, and always shown back to the person so they can correct it.
 */
internal object FreeAsk {
    private val LIVES: List<Pair<String, List<String>>> = listOf(
        "Love" to listOf("love", "marriage", "married", "marry", "wife", "husband", "spouse", "partner", "girlfriend", "boyfriend", "divorce", "separat", "relationship", "wedding", "proposal",
            "പ്രണയ", "വിവാഹ", "ഭാര്യ", "ഭർത്താവ", "കല്യാണ", "ബന്ധം", "शादी", "विवाह", "पत्नी", "पति", "प्यार", "प्रेम", "रिश्त", "तलाक"),
        "Career" to listOf("job", "work", "career", "boss", "promotion", "office", "interview", "transfer", "employ", "resign", "ജോലി", "ഓഫീസ", "സ്ഥലംമാറ്റ", "നൗകരി", "नौकरी", "काम", "करियर", "प्रमोशन", "दफ्तर"),
        "Money" to listOf("money", "loan", "debt", "salary", "income", "business", "profit", "loss", "invest", "pay", "പണ", "കടം", "ശമ്പള", "ബിസിനസ", "लाभ", "पैसा", "कर्ज", "वेतन", "व्यापार", "धन"),
        "Health" to listOf("health", "ill", "sick", "disease", "surgery", "operation", "hospital", "pain", "doctor", "recover", "ആരോഗ്യ", "അസുഖ", "രോഗ", "ആശുപത്രി", "स्वास्थ्य", "बीमार", "रोग", "अस्पताल", "इलाज"),
        "Family" to listOf("family", "mother", "father", "parents", "brother", "sister", "in-law", "relative", "കുടുംബ", "അമ്മ", "അച്ഛ", "സഹോദര", "परिवार", "माँ", "पिता", "भाई", "बहन"),
        "Study" to listOf("study", "exam", "course", "college", "school", "result", "degree", "admission", "പഠന", "പരീക്ഷ", "കോളേജ", "पढ़ाई", "परीक्षा", "कॉलेज", "एडमिशन"),
        "Home" to listOf("house", "home", "land", "property", "rent", "flat", "apartment", "plot", "വീട", "ഭൂമി", "സ്ഥലം", "വാടക", "घर", "ज़मीन", "जमीन", "मकान", "किराया"),
        "Travel" to listOf("travel", "abroad", "visa", "trip", "journey", "foreign", "gulf", "settle", "യാത്ര", "വിദേശ", "വിസ", "गल्फ", "यात्रा", "विदेश", "वीज़ा"),
        "Children" to listOf("child", "children", "baby", "pregnan", "son", "daughter", "kid", "conceive", "കുട്ടി", "കുഞ്ഞ", "മകൻ", "മകൾ", "ഗർഭ", "बच्च", "बेटा", "बेटी", "संतान", "गर्भ"),
    )
    private val QUESTIONS: List<Pair<String, List<String>>> = listOf(
        "will_it_return" to listOf("come back", "return", "reunite", "back to me", "തിരിച്ചു", "മടങ്ങി", "वापस", "लौट"),
        "will_it_recover" to listOf("set right", "recover", "heal", "get better", "fixed", "improve", "ശരിയാകു", "മെച്ചപ്പെ", "സുഖ", "ठीक", "सुधर"),
        "when" to listOf("when", "how long", "which month", "which year", "എപ്പോൾ", "എന്ന്", "എത്ര കാലം", "कब", "कितने समय"),
        "good_time_now" to listOf("now", "today", "this week", "right time", "good time", "ഇപ്പോൾ", "ഇന്ന്", "നല്ല സമയ", "अभी", "आज", "सही समय"),
        "should_i" to listOf("should i", "is it wise", "shall i", "go ahead", "ചെയ്യണോ", "വേണോ", "करूँ", "करना चाहिए", "चाहिए"),
    )

    fun understand(text: String, topics: List<TopicRecord>): Understood {
        val t = text.lowercase()
        val life = LIVES.maxByOrNull { (_, ws) -> ws.count { it in t } }?.takeIf { (_, ws) -> ws.any { it in t } }?.first ?: "Love"
        val q = QUESTIONS.firstOrNull { (_, ws) -> ws.any { it in t } }?.first ?: "will_it_happen"
        // The matter: the topic under that life whose words share the most with the question.
        val words = t.split(Regex("[^\\p{L}\\p{N}]+")).filter { it.length > 3 }.toSet()
        val topic = topics.filter { it.life == life }
            .map { tp -> tp to tp.words.lowercase().split(Regex("[^\\p{L}\\p{N}]+")).count { w -> w.length > 3 && w in words } }
            .filter { it.second >= 1 }
            .maxByOrNull { it.second }?.first
        return Understood(life, topic, q)
    }
}
