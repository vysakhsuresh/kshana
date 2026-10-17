package com.kshana.app.system

import android.content.Context
import android.os.Bundle
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.util.Locale

/**
 * The mantra of the hour, spoken slowly by the phone's own voice, counted to 108. No
 * audio files, no network. Stop at any count; the count is shown as it goes.
 */
object Chant {
    private var tts: TextToSpeech? = null
    private var ready = false
    var count by mutableIntStateOf(0)
        private set
    var running by mutableStateOf(false)
        private set
    private var text: String = ""
    private var target = 108

    private fun ensure(context: Context, then: () -> Unit) {
        if (tts != null && ready) { then(); return }
        tts = TextToSpeech(context.applicationContext) { status ->
            ready = status == TextToSpeech.SUCCESS
            if (ready) {
                tts?.language = Locale("hi", "IN")
                tts?.setSpeechRate(0.7f)
                tts?.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
                    override fun onStart(utteranceId: String?) {}
                    override fun onError(utteranceId: String?) { running = false }
                    override fun onDone(utteranceId: String?) {
                        if (!running) return
                        count += 1
                        if (count >= target) { running = false; return }
                        speakOnce()
                    }
                })
                then()
            }
        }
    }

    private fun speakOnce() {
        val params = Bundle()
        tts?.speak(text, TextToSpeech.QUEUE_FLUSH, params, "kshana-chant-$count")
    }

    /** Begin a round of `times` (108 by default). */
    fun start(context: Context, mantra: String, times: Int = 108) {
        text = mantra
        target = times
        count = 0
        running = true
        ensure(context) { speakOnce() }
    }

    fun stop() {
        running = false
        tts?.stop()
    }
}
