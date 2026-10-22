package com.kshana.app.data

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/**
 * The app's own words in English, Malayalam and Hindi: tabs, lives, bands, labels,
 * buttons, the questions and the purposes. The engine's sentences (reasons, headlines,
 * remedies) stay in English until their templates are translated; Settings says so.
 */
object Lang {
    private const val KEY = "settings.lang"
    var code by mutableStateOf("en")
        private set

    fun load(context: Context) {
        code = ProfileStore.prefs(context).getString(KEY, "en") ?: "en"
    }

    fun set(context: Context, c: String) {
        code = c
        ProfileStore.prefs(context).edit().putString(KEY, c).apply()
    }

    val choices = listOf("en" to "English", "ml" to "മലയാളം", "hi" to "हिन्दी")

    private val ml: Map<String, String> = mapOf(
        "LOOP" to "ലൂപ്പ്", "NOW" to "ഇപ്പോൾ", "AHEAD" to "മുന്നോട്ട്", "FUTURE" to "ഭാവി", "READING" to "വായന", "ASK" to "ചോദിക്കൂ", "PLAN" to "പ്ലാൻ", "US" to "നമ്മൾ", "MORE" to "കൂടുതൽ", "EXPLORE" to "കണ്ടെത്തൂ",
        "Love" to "പ്രണയം", "Career" to "ജോലി", "Money" to "പണം", "Health" to "ആരോഗ്യം", "Family" to "കുടുംബം",
        "Study" to "പഠനം", "Home" to "വീട്", "Travel" to "യാത്ര", "Children" to "മക്കൾ",
        "Bright" to "തിളക്കം", "Good" to "നല്ലത്", "Steady" to "സ്ഥിരം", "Slow" to "മന്ദം", "Hold" to "കാത്തിരിക്കൂ", "Wait" to "കാത്തിരിക്കൂ",
        "Good morning" to "സുപ്രഭാതം", "Good afternoon" to "ശുഭ മധ്യാഹ്നം", "Good evening" to "ശുഭ സായാഹ്നം", "Good night" to "ശുഭ രാത്രി",
        "Your life · right now" to "നിങ്ങളുടെ ജീവിതം · ഇപ്പോൾ",
        "The hours of today" to "ഇന്നത്തെ മണിക്കൂറുകൾ", "Watch the clock" to "സമയം ശ്രദ്ധിക്കൂ", "Coming up" to "വരാനിരിക്കുന്നത്",
        "Under the hood" to "അകത്ത്", "changes in" to "മാറുന്നത്", "TRAVEL" to "യാത്ര", "TRAVELLING" to "യാത്രയിൽ",
        "REMEDIES" to "പരിഹാരങ്ങൾ", "PLAN A TIME" to "ഒരു സമയം നോക്കൂ", "THE TWO OF US" to "നമ്മൾ രണ്ടുപേർ",
        "THE BOOK OF YOUR LIFE · PAST, PRESENT, FUTURE" to "നിങ്ങളുടെ ജീവിതപുസ്തകം · ഭൂതം, വർത്തമാനം, ഭാവി",
        "Ask the sky" to "ആകാശത്തോട് ചോദിക്കൂ", "About" to "എന്തിനെക്കുറിച്ച്", "Question" to "ചോദ്യം",
        "How is that time for me?" to "ആ സമയം എനിക്ക് എങ്ങനെ?", "When" to "എപ്പോൾ", "For" to "എന്തിന്",
        "The two of you" to "നിങ്ങൾ രണ്ടുപേർ", "With" to "ആരുമായി",
        "Will it happen?" to "അത് സംഭവിക്കുമോ?", "When?" to "എപ്പോൾ?", "Is now a good time?" to "ഇപ്പോൾ നല്ല സമയമാണോ?",
        "Should I?" to "ഞാൻ ചെയ്യണോ?", "Will it come back?" to "അത് തിരിച്ചുവരുമോ?", "Will it be set right?" to "അത് ശരിയാകുമോ?",
        "Meeting someone" to "ഒരാളെ കാണാൻ", "Interview, pitch" to "ഇന്റർവ്യൂ", "Exam, test" to "പരീക്ഷ", "Journey" to "യാത്ര",
        "Deal, signing, purchase" to "കരാർ, ഒപ്പ്, വാങ്ങൽ", "Doctor, treatment" to "ഡോക്ടർ, ചികിത്സ", "Date, proposal" to "പ്രണയം, വിവാഹാലോചന",
        "Family matter" to "കുടുംബകാര്യം", "Home, land, rent" to "വീട്, ഭൂമി, വാടക", "Starting something new" to "പുതിയ തുടക്കം", "Anything at all" to "എന്തും",
        "Everything is worked out on this phone from your birth time. Nothing leaves it." to "എല്ലാം ഈ ഫോണിൽ നിങ്ങളുടെ ജനനസമയത്തിൽ നിന്ന് കണക്കാക്കുന്നു. ഒന്നും പുറത്തുപോകുന്നില്ല.",
    )

    private val hi: Map<String, String> = mapOf(
        "LOOP" to "लूप", "NOW" to "अभी", "AHEAD" to "आगे", "FUTURE" to "भविष्य", "READING" to "पाठ", "ASK" to "पूछें", "PLAN" to "योजना", "US" to "हम", "MORE" to "और", "EXPLORE" to "खोजें",
        "Love" to "प्रेम", "Career" to "करियर", "Money" to "धन", "Health" to "स्वास्थ्य", "Family" to "परिवार",
        "Study" to "पढ़ाई", "Home" to "घर", "Travel" to "यात्रा", "Children" to "संतान",
        "Bright" to "उज्ज्वल", "Good" to "अच्छा", "Steady" to "स्थिर", "Slow" to "धीमा", "Hold" to "रुकें", "Wait" to "रुकें",
        "Good morning" to "सुप्रभात", "Good afternoon" to "शुभ दोपहर", "Good evening" to "शुभ संध्या", "Good night" to "शुभ रात्रि",
        "Your life · right now" to "आपका जीवन · अभी",
        "The hours of today" to "आज के घंटे", "Watch the clock" to "घड़ी देखें", "Coming up" to "आगे",
        "Under the hood" to "भीतर", "changes in" to "बदलेगा", "TRAVEL" to "यात्रा", "TRAVELLING" to "यात्रा में",
        "REMEDIES" to "उपाय", "PLAN A TIME" to "समय देखें", "THE TWO OF US" to "हम दोनों",
        "THE BOOK OF YOUR LIFE · PAST, PRESENT, FUTURE" to "आपके जीवन की पुस्तक · भूत, वर्तमान, भविष्य",
        "Ask the sky" to "आकाश से पूछें", "About" to "किस बारे में", "Question" to "प्रश्न",
        "How is that time for me?" to "वह समय मेरे लिए कैसा है?", "When" to "कब", "For" to "किसलिए",
        "The two of you" to "आप दोनों", "With" to "किसके साथ",
        "Will it happen?" to "क्या यह होगा?", "When?" to "कब?", "Is now a good time?" to "क्या अभी अच्छा समय है?",
        "Should I?" to "क्या मैं करूँ?", "Will it come back?" to "क्या यह लौटेगा?", "Will it be set right?" to "क्या यह ठीक होगा?",
        "Meeting someone" to "किसी से मिलना", "Interview, pitch" to "इंटरव्यू", "Exam, test" to "परीक्षा", "Journey" to "यात्रा",
        "Deal, signing, purchase" to "सौदा, हस्ताक्षर, खरीद", "Doctor, treatment" to "डॉक्टर, इलाज", "Date, proposal" to "मुलाक़ात, प्रस्ताव",
        "Family matter" to "पारिवारिक बात", "Home, land, rent" to "घर, ज़मीन, किराया", "Starting something new" to "नई शुरुआत", "Anything at all" to "कुछ भी",
        "Everything is worked out on this phone from your birth time. Nothing leaves it." to "सब कुछ इसी फ़ोन पर आपके जन्म समय से गिना जाता है। कुछ भी बाहर नहीं जाता।",
    )

    private val mlAll: Map<String, String> by lazy { LangExplore.ml + LangTrust.ml + LangTrust2.ml + ml }
    private val hiAll: Map<String, String> by lazy { LangExplore.hi + LangTrust.hi + LangTrust2.hi + hi }

    /** The word in the chosen language, or the English when there is none yet. */
    fun t(s: String): String = when (code) {
        "ml" -> mlAll[s] ?: s
        "hi" -> hiAll[s] ?: s
        else -> s
    }

    /**
     * A sentence with blanks: the English [template] uses {0}, {1}… for the parts that change
     * ("Keep it {0} Wednesdays"). The template is translated first, then the blanks are filled.
     */
    fun f(template: String, vararg args: Any): String {
        var out = t(template)
        args.forEachIndexed { i, a -> out = out.replace("{$i}", a.toString()) }
        return out
    }
}
