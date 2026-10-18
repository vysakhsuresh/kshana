package com.kshana.app.system

import android.content.Context
import android.speech.tts.TextToSpeech
import java.util.Locale

/**
 * Reads a passage aloud with the phone's own voice, in the app's language when the
 * phone has that voice, else in Indian English. No recording, no network.
 */
object Speak {
    private var tts: TextToSpeech? = null
    private var ready = false
    private var pending: String? = null

    fun say(context: Context, text: String, lang: String) {
        val t = tts
        if (t != null && ready) {
            speakNow(t, text, lang)
            return
        }
        pending = text
        tts = TextToSpeech(context.applicationContext) { status ->
            ready = status == TextToSpeech.SUCCESS
            val p = pending
            if (ready && p != null) {
                pending = null
                tts?.let { speakNow(it, p, lang) }
            }
        }
    }

    private fun speakNow(t: TextToSpeech, text: String, lang: String) {
        val wanted = when (lang) {
            "ml" -> Locale("ml", "IN")
            "hi" -> Locale("hi", "IN")
            else -> Locale("en", "IN")
        }
        val ok = t.isLanguageAvailable(wanted) >= TextToSpeech.LANG_AVAILABLE
        t.language = if (ok) wanted else Locale("en", "IN")
        t.setSpeechRate(0.92f)
        t.speak(text, TextToSpeech.QUEUE_FLUSH, null, "kshana-say")
    }

    fun stop() {
        tts?.stop()
    }

    fun speaking(): Boolean = tts?.isSpeaking == true
}
