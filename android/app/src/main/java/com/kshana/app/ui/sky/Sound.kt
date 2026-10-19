package com.kshana.app.ui.sky

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.os.SystemClock
import com.kshana.app.data.ProfileStore
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.pow
import kotlin.math.sin

/*
 * Generated ambient sound with no audio assets, synthesised into an AudioTrack:
 *
 *   drone  two detuned sines (f and f·1.004) over a sub-octave, breathing at 0.1 Hz, run
 *          through a one-pole low-pass whose cutoff and gain fall with the sky brightness
 *          (night A2 110 Hz, day A3 220 Hz, gliding over about a minute; day is quieter
 *          and darker).
 *   bell   a decaying sum of three partials (f, 2.0f, 2.76f: the hum, prime and tierce of a
 *          church bell) with decays of 1.2, 0.9 and 0.6 s; the kind sets the strike note.
 *
 * Off by default (prefs key "ui.sound"); honours silent mode and audio focus; stops when
 * the UI is hidden (via [Foreground]) and 20 minutes after the last interaction. Output
 * latency is 20–45 ms on most phones, so a chime asked for at an instant sounds that much
 * after it; the engine's instant stays exact in print.
 */
object SkySound {
    const val PREF_SOUND = "ui.sound"

    private const val IDLE_STOP_MS = 20L * 60L * 1000L
    private const val MAX_BELLS = 4
    private const val BELL_SECONDS = 3.0

    private val running = AtomicBoolean(false)
    @Volatile private var track: AudioTrack? = null
    @Volatile private var thread: Thread? = null
    @Volatile private var focusRequest: AudioFocusRequest? = null
    @Volatile private var audioManager: AudioManager? = null
    @Volatile private var droneOn = false
    @Volatile private var daylight = 0f
    @Volatile private var ducked = false
    @Volatile private var lastTouch = 0L
    @Volatile private var hiddenHooked = false
    /** Set when the drone stopped after 20 idle minutes; cleared by the next interaction. */
    @Volatile private var idle = false

    /** Pending bells, written by any thread and consumed by the render thread. */
    private val bellLock = Any()
    private val bellFreq = DoubleArray(MAX_BELLS)
    private val bellGain = FloatArray(MAX_BELLS)
    private val bellStart = DoubleArray(MAX_BELLS)   // seconds on the render clock
    private val bellLive = BooleanArray(MAX_BELLS)
    private var pendingFreq = 0.0
    private var pendingGain = 0f
    private var pendingCount = 0

    val isRunning: Boolean get() = running.get()

    /** The app switch (default off). */
    fun enabled(context: Context): Boolean =
        runCatching { ProfileStore.prefs(context).getBoolean(PREF_SOUND, false) }.getOrDefault(false)

    fun setEnabled(context: Context, on: Boolean) {
        runCatching { ProfileStore.prefs(context).edit().putBoolean(PREF_SOUND, on).apply() }
        if (!on) stop()
    }

    /** Start the drone (if the switch is on, the phone is not silent and focus is granted). */
    fun start(context: Context) {
        if (!enabled(context)) return
        droneOn = true
        touch()
        startEngine(context, AudioManager.AUDIOFOCUS_GAIN)
    }

    /** Stop everything and release focus. Safe from any thread, idempotent. */
    fun stop() {
        droneOn = false
        if (!running.compareAndSet(true, false)) return
        val t = thread
        thread = null
        runCatching { t?.join(500) }
        releaseTrack()
        releaseFocus()
    }

    /**
     * Strike the bell for an exact instant of [kind]. With the drone running it joins the
     * mix; otherwise, given a [context] and the switch on, a short bell-only session starts
     * and ends when the bell has decayed.
     */
    fun chime(kind: String, context: Context? = null) {
        if (!running.get() && (context == null || !enabled(context))) return
        // Queue first so a bell-only session's first block already carries the bell.
        synchronized(bellLock) {
            pendingFreq = bellNote(kind)
            pendingGain = 0.6f
            pendingCount++
        }
        if (!running.get() && context != null) {
            startEngine(context, AudioManager.AUDIOFOCUS_GAIN_TRANSIENT_MAY_DUCK)
            if (!running.get()) synchronized(bellLock) { pendingCount = 0 }
        }
    }

    /**
     * Start the drone again after the UI came back, unless it stopped for idleness; the
     * sky calls this every few seconds, so a restart never needs a lifecycle hook.
     */
    fun resume(context: Context) {
        if (running.get() || idle) return
        start(context)
    }

    /** Feed the sky: solar altitude in degrees drives the drone's brightness and pitch. */
    fun setSky(sunAltitudeDeg: Double) {
        daylight = ((sunAltitudeDeg + 6.0) / 12.0).coerceIn(0.0, 1.0).toFloat()
    }

    /** Note an interaction; the drone stops 20 minutes after the last one. */
    fun touch() {
        lastTouch = SystemClock.elapsedRealtime()
        idle = false
    }

    /** The strike note per kind: dasha low, Moon and hora mid, sunrise bright, the rest in between. */
    private fun bellNote(kind: String): Double = when (kind.trim().lowercase()) {
        "dasha", "maha", "antar", "mahadasha", "antardasha", "dasha.major" -> 220.0
        "dasha.minor", "minor", "pratyantar", "sookshma", "prana" -> 293.66
        "sunrise", "rise", "dawn" -> 659.26
        "sunset", "set", "dusk" -> 329.63
        "hora" -> 493.88
        "muhurta", "window" -> 587.33
        "rahu", "rahukala", "rahu kala" -> 246.94
        "no", "reject" -> 311.13
        else -> 440.0 // moon, sub, yes, unknown
    }

    private fun startEngine(context: Context, focusGain: Int) {
        if (running.get()) return
        val app = context.applicationContext
        val am = app.getSystemService(Context.AUDIO_SERVICE) as? AudioManager ?: return
        if (runCatching { am.ringerMode != AudioManager.RINGER_MODE_NORMAL }.getOrDefault(false)) return
        if (!running.compareAndSet(false, true)) return
        audioManager = am
        Foreground.register(app)
        if (!hiddenHooked) {
            hiddenHooked = true
            Foreground.addOnHidden { stop() }
        }
        if (!requestFocus(am, focusGain)) {
            running.set(false)
            return
        }
        val rate = runCatching { am.getProperty(AudioManager.PROPERTY_OUTPUT_SAMPLE_RATE)?.toInt() }.getOrNull()?.takeIf { it in 8000..192000 } ?: 48000
        val framesPerBuffer = runCatching { am.getProperty(AudioManager.PROPERTY_OUTPUT_FRAMES_PER_BUFFER)?.toInt() }.getOrNull()?.takeIf { it in 32..8192 } ?: 256
        val block = maxOf(framesPerBuffer, 256)
        val minBytes = runCatching { AudioTrack.getMinBufferSize(rate, AudioFormat.CHANNEL_OUT_MONO, AudioFormat.ENCODING_PCM_FLOAT) }.getOrDefault(0)
        val bufferBytes = maxOf(minBytes, 4 * block * 4)
        val built = runCatching {
            AudioTrack.Builder()
                .setAudioAttributes(mediaAttributes())
                .setAudioFormat(
                    AudioFormat.Builder()
                        .setEncoding(AudioFormat.ENCODING_PCM_FLOAT)
                        .setSampleRate(rate)
                        .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                        .build(),
                )
                .setBufferSizeInBytes(bufferBytes)
                .setTransferMode(AudioTrack.MODE_STREAM)
                .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_POWER_SAVING)
                .build()
        }.getOrNull()
        if (built == null || built.state != AudioTrack.STATE_INITIALIZED) {
            runCatching { built?.release() }
            releaseFocus()
            running.set(false)
            return
        }
        track = built
        touch()
        val t = Thread({ render(built, rate, block) }, "kshana-sky-sound").apply { isDaemon = true; priority = Thread.NORM_PRIORITY }
        thread = t
        t.start()
    }

    private fun mediaAttributes(): AudioAttributes = AudioAttributes.Builder()
        .setUsage(AudioAttributes.USAGE_MEDIA)
        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
        .build()

    private fun requestFocus(am: AudioManager, gain: Int): Boolean = runCatching {
        val request = AudioFocusRequest.Builder(gain)
            .setAudioAttributes(mediaAttributes())
            .setWillPauseWhenDucked(false)
            .setOnAudioFocusChangeListener { change ->
                when (change) {
                    AudioManager.AUDIOFOCUS_LOSS, AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> stop()
                    AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK -> ducked = true
                    AudioManager.AUDIOFOCUS_GAIN -> ducked = false
                }
            }
            .build()
        focusRequest = request
        am.requestAudioFocus(request) == AudioManager.AUDIOFOCUS_REQUEST_GRANTED
    }.getOrDefault(false)

    private fun releaseFocus() {
        val am = audioManager
        val r = focusRequest
        focusRequest = null
        if (am != null && r != null) runCatching { am.abandonAudioFocusRequest(r) }
    }

    private fun releaseTrack() {
        val t = track
        track = null
        if (t != null) runCatching {
            t.pause()
            t.flush()
            t.release()
        }
    }

    /** The render loop: one block at a time, blocking on the track; no allocation inside. */
    private fun render(out: AudioTrack, rate: Int, block: Int) {
        val buf = FloatArray(block)
        val twoPi = 2.0 * PI
        var phaseA = 0.0
        var phaseB = 0.0
        var phaseSub = 0.0
        var tonic = 110.0 * 2.0.pow(daylight.toDouble())
        var lowPass = 0f
        var gain = 0f
        var clock = 0.0 // seconds rendered
        val bellPhase = DoubleArray(MAX_BELLS * 3)
        val glide = (block.toDouble() / (60.0 * rate)).coerceIn(0.0, 1.0) // tonic reaches its target in about a minute

        runCatching {
            // Pre-roll one block of silence to skip the warm-up.
            out.play()
            out.write(buf, 0, block, AudioTrack.WRITE_BLOCKING)
        }.onFailure {
            running.set(false)
        }

        while (running.get()) {
            // Take new bells.
            synchronized(bellLock) {
                while (pendingCount > 0) {
                    pendingCount--
                    var slot = -1
                    for (i in 0 until MAX_BELLS) if (!bellLive[i]) { slot = i; break }
                    if (slot < 0) slot = 0
                    bellLive[slot] = true
                    bellFreq[slot] = pendingFreq
                    bellGain[slot] = pendingGain
                    bellStart[slot] = clock
                    bellPhase[slot * 3] = 0.0; bellPhase[slot * 3 + 1] = 0.0; bellPhase[slot * 3 + 2] = 0.0
                }
            }
            val day = daylight
            val droneTarget = if (droneOn) (if (ducked) 0.05f else 0.16f) * (1f - 0.65f * day) else 0f
            val tonicTarget = 110.0 * 2.0.pow(day.toDouble())
            tonic += (tonicTarget - tonic) * glide
            val cutoff = 220.0 + 700.0 * (1.0 - day)
            val lpCoef = (1.0 - exp(-twoPi * cutoff / rate)).toFloat()
            val stepA = twoPi * tonic / rate
            val stepB = twoPi * tonic * 1.004 / rate
            val stepSub = twoPi * tonic * 0.5 / rate
            val gainStep = (droneTarget - gain) / block

            var anyBell = false
            for (n in 0 until block) {
                val t = clock + n.toDouble() / rate
                gain += gainStep
                // Drone: breathe at 0.1 Hz with the sky.
                val breath = 0.7f + 0.3f * sin(twoPi * 0.1 * t).toFloat()
                var s = 0f
                if (gain > 0.0005f) {
                    val raw = (sin(phaseA) + sin(phaseB) + 0.5 * sin(phaseSub)).toFloat() * 0.4f
                    lowPass += lpCoef * (raw - lowPass)
                    s = lowPass * gain * breath
                }
                phaseA += stepA; if (phaseA > twoPi) phaseA -= twoPi
                phaseB += stepB; if (phaseB > twoPi) phaseB -= twoPi
                phaseSub += stepSub; if (phaseSub > twoPi) phaseSub -= twoPi
                // Bells: three partials with their own decays.
                for (b in 0 until MAX_BELLS) {
                    if (!bellLive[b]) continue
                    val age = t - bellStart[b]
                    if (age > BELL_SECONDS) { bellLive[b] = false; continue }
                    anyBell = true
                    val f = bellFreq[b]
                    val g = bellGain[b] * (if (ducked) 0.5f else 1f)
                    val e1 = exp(-age / 1.2).toFloat()
                    val e2 = exp(-age / 0.9).toFloat()
                    val e3 = exp(-age / 0.6).toFloat()
                    val attack = if (age < 0.004) (age / 0.004).toFloat() else 1f
                    val p = b * 3
                    s += g * attack * (1.0f * e1 * sin(bellPhase[p]).toFloat() + 0.5f * e2 * sin(bellPhase[p + 1]).toFloat() + 0.35f * e3 * sin(bellPhase[p + 2]).toFloat()) * 0.5f
                    bellPhase[p] += twoPi * f / rate
                    bellPhase[p + 1] += twoPi * f * 2.0 / rate
                    bellPhase[p + 2] += twoPi * f * 2.76 / rate
                    if (bellPhase[p] > twoPi) bellPhase[p] -= twoPi
                    if (bellPhase[p + 1] > twoPi) bellPhase[p + 1] -= twoPi
                    if (bellPhase[p + 2] > twoPi) bellPhase[p + 2] -= twoPi
                }
                // Soft clip.
                buf[n] = s / (1f + abs(s))
            }
            clock += block.toDouble() / rate
            val written = runCatching { out.write(buf, 0, block, AudioTrack.WRITE_BLOCKING) }.getOrDefault(-1)
            if (written < 0) break
            // Bell-only session ends when the bell has decayed; the drone ends after 20 idle minutes.
            if (!droneOn && !anyBell && gain < 0.001f && clock > 0.25) break
            if (droneOn && SystemClock.elapsedRealtime() - lastTouch > IDLE_STOP_MS) {
                idle = true
                droneOn = false
                break
            }
        }
        if (running.compareAndSet(true, false)) {
            thread = null
            releaseTrack()
            releaseFocus()
        }
    }
}
