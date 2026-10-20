package com.kshana.app.ui.sky

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.media.AudioAttributes
import android.os.Build
import android.os.VibrationAttributes
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import android.provider.Settings
import android.view.HapticFeedbackConstants
import android.view.View
import com.kshana.app.data.ProfileStore

/*
 * The haptic language of exact instants: one phrase per transition kind, three intensity
 * levels (0.5, 0.7, 1.0; ratio ≥ 1.4), gaps of at least 50 ms, and the Android fallback
 * ladder:
 *
 *   API 30+  VibrationEffect.Composition primitives, only when the vibrator supports every
 *            primitive of the phrase (a composition with one unsupported primitive fails
 *            entirely); THUD and LOW_TICK are API 31 and are substituted on 30.
 *   API 29   createPredefined (EFFECT_TICK / CLICK / HEAVY_CLICK / DOUBLE_CLICK) when the
 *            vibrator has no amplitude control.
 *   API 26+  createWaveform / createOneShot with amplitudes (on/off where unsupported).
 *   last     View.performHapticFeedback on the activity's decor view (no VIBRATE needed).
 *
 * Honours the app's "ui.haptics" switch (default on) and the system haptic-feedback
 * setting. For an instant fired from the background (an alarm), pass `alarm = true` so the
 * vibration carries USAGE_ALARM and is not dropped.
 */
object Haptics {
    const val PREF_HAPTICS = "ui.haptics"

    /** Scale levels: the faintest perceptible is 0, so 0.5 / 0.7 / 1.0 keep the 1.4 ratio. */
    private const val SOFT = 0.5f
    private const val MID = 0.7f
    private const val FULL = 1f

    // Primitive ids as VibrationEffect.Composition defines them (compile-time constants).
    private const val P_CLICK = VibrationEffect.Composition.PRIMITIVE_CLICK
    private const val P_THUD = VibrationEffect.Composition.PRIMITIVE_THUD
    private const val P_QUICK_RISE = VibrationEffect.Composition.PRIMITIVE_QUICK_RISE
    private const val P_SLOW_RISE = VibrationEffect.Composition.PRIMITIVE_SLOW_RISE
    private const val P_QUICK_FALL = VibrationEffect.Composition.PRIMITIVE_QUICK_FALL
    private const val P_TICK = VibrationEffect.Composition.PRIMITIVE_TICK
    private const val P_LOW_TICK = VibrationEffect.Composition.PRIMITIVE_LOW_TICK

    private class Prim(val id: Int, val scale: Float, val delayMs: Int)

    /**
     * One phrase: its primitives (API 30+), the waveform for 26–29 (timings alternate
     * off/on starting with off, amplitudes 0–255), the predefined effect for vibrators
     * without amplitude control on 29+, and the view constant for the last rung.
     */
    private class Phrase(
        val prims: Array<Prim>,
        val timings: LongArray,
        val amplitudes: IntArray,
        val predefined: Int,
        val viewConstant: Int,
        val viewConstantLegacy: Int,
    )

    private val CONFIRM = if (Build.VERSION.SDK_INT >= 30) HapticFeedbackConstants.CONFIRM else HapticFeedbackConstants.VIRTUAL_KEY
    private val REJECT = if (Build.VERSION.SDK_INT >= 30) HapticFeedbackConstants.REJECT else HapticFeedbackConstants.VIRTUAL_KEY

    private val DASHA = Phrase(
        arrayOf(Prim(P_SLOW_RISE, MID, 0), Prim(P_THUD, FULL, 0)),
        longArrayOf(0, 90, 20, 70), intArrayOf(0, 110, 0, 255),
        VibrationEffect.EFFECT_HEAVY_CLICK, CONFIRM, HapticFeedbackConstants.LONG_PRESS,
    )
    private val DASHA_MINOR = Phrase(
        arrayOf(Prim(P_CLICK, MID, 0)),
        longArrayOf(0, 25), intArrayOf(0, 180),
        VibrationEffect.EFFECT_CLICK, HapticFeedbackConstants.VIRTUAL_KEY, HapticFeedbackConstants.VIRTUAL_KEY,
    )
    private val MOON = Phrase(
        arrayOf(Prim(P_TICK, SOFT, 0)),
        longArrayOf(0, 15), intArrayOf(0, 130),
        VibrationEffect.EFFECT_TICK, HapticFeedbackConstants.CLOCK_TICK, HapticFeedbackConstants.CLOCK_TICK,
    )
    private val SUB = Phrase(
        arrayOf(Prim(P_LOW_TICK, SOFT, 0)),
        longArrayOf(0, 12), intArrayOf(0, 100),
        VibrationEffect.EFFECT_TICK, HapticFeedbackConstants.CLOCK_TICK, HapticFeedbackConstants.CLOCK_TICK,
    )
    private val HORA = Phrase(
        arrayOf(Prim(P_TICK, SOFT, 0), Prim(P_TICK, SOFT, 60)),
        longArrayOf(0, 15, 60, 15), intArrayOf(0, 130, 0, 130),
        VibrationEffect.EFFECT_DOUBLE_CLICK, HapticFeedbackConstants.CLOCK_TICK, HapticFeedbackConstants.CLOCK_TICK,
    )
    private val SUNRISE = Phrase(
        arrayOf(Prim(P_SLOW_RISE, FULL, 0), Prim(P_CLICK, MID, 0)),
        longArrayOf(0, 60, 60, 60, 60, 60, 20, 25), intArrayOf(0, 40, 90, 140, 200, 255, 0, 200),
        VibrationEffect.EFFECT_HEAVY_CLICK, CONFIRM, HapticFeedbackConstants.VIRTUAL_KEY,
    )
    private val SUNSET = Phrase(
        arrayOf(Prim(P_QUICK_FALL, MID, 0)),
        longArrayOf(0, 50, 50, 50, 50), intArrayOf(0, 255, 170, 90, 40),
        VibrationEffect.EFFECT_CLICK, HapticFeedbackConstants.VIRTUAL_KEY, HapticFeedbackConstants.VIRTUAL_KEY,
    )
    private val MUHURTA = Phrase(
        arrayOf(Prim(P_QUICK_RISE, MID, 0), Prim(P_CLICK, FULL, 0)),
        longArrayOf(0, 40, 40, 10, 30), intArrayOf(0, 120, 230, 0, 255),
        VibrationEffect.EFFECT_HEAVY_CLICK, CONFIRM, HapticFeedbackConstants.VIRTUAL_KEY,
    )
    private val RAHU = Phrase(
        arrayOf(Prim(P_LOW_TICK, MID, 0), Prim(P_LOW_TICK, SOFT, 80)),
        longArrayOf(0, 15, 80, 15), intArrayOf(0, 160, 0, 110),
        VibrationEffect.EFFECT_DOUBLE_CLICK, HapticFeedbackConstants.CLOCK_TICK, HapticFeedbackConstants.CLOCK_TICK,
    )
    private val YES = Phrase(
        arrayOf(Prim(P_CLICK, MID, 0), Prim(P_TICK, SOFT, 50)),
        longArrayOf(0, 20, 50, 12), intArrayOf(0, 200, 0, 120),
        VibrationEffect.EFFECT_CLICK, CONFIRM, HapticFeedbackConstants.VIRTUAL_KEY,
    )
    private val NO = Phrase(
        arrayOf(Prim(P_TICK, SOFT, 0), Prim(P_TICK, SOFT, 50), Prim(P_TICK, SOFT, 50)),
        longArrayOf(0, 12, 50, 12, 50, 12), intArrayOf(0, 120, 0, 120, 0, 120),
        VibrationEffect.EFFECT_DOUBLE_CLICK, REJECT, HapticFeedbackConstants.VIRTUAL_KEY,
    )

    /**
     * The phrase for a transition kind. Kinds (case-insensitive, synonyms accepted):
     * `dasha` (maha/antar: a slow rise into a thud), `dasha.minor` (pratyantar and below:
     * a click), `moon` (nakshatra, tithi, sign: a tick), `sub` (KP sub: a low tick), `hora`
     * (a double tick), `sunrise` (a swell into a click), `sunset` (a quick fall), `muhurta`
     * (a quick rise into a click), `rahu` (two low ticks), `yes`, `no`. Unknown kinds tick.
     */
    private fun phrase(kind: String): Phrase = when (kind.trim().lowercase()) {
        "dasha", "maha", "antar", "mahadasha", "antardasha", "dasha.major" -> DASHA
        "dasha.minor", "minor", "pratyantar", "sookshma", "prana" -> DASHA_MINOR
        "moon", "nakshatra", "tithi", "sign", "ingress", "tick" -> MOON
        "sub", "kp", "kp.sub", "subsub", "sub-sub", "kp.subsub" -> SUB
        "hora" -> HORA
        "sunrise", "rise", "dawn" -> SUNRISE
        "sunset", "set", "dusk" -> SUNSET
        "muhurta", "window" -> MUHURTA
        "rahu", "rahukala", "rahu kala" -> RAHU
        "yes", "confirm" -> YES
        "no", "reject" -> NO
        else -> MOON
    }

    /** The app switch (default on) and the system haptic-feedback setting, both honoured. */
    fun enabled(context: Context): Boolean {
        val app = runCatching { ProfileStore.prefs(context).getBoolean(PREF_HAPTICS, true) }.getOrDefault(true)
        if (!app) return false
        return runCatching {
            Settings.System.getInt(context.contentResolver, Settings.System.HAPTIC_FEEDBACK_ENABLED, 1) != 0
        }.getOrDefault(true)
    }

    fun setEnabled(context: Context, on: Boolean) {
        runCatching { ProfileStore.prefs(context).edit().putBoolean(PREF_HAPTICS, on).apply() }
    }

    /**
     * The phrase for an exact instant of [kind]. Schedule it ~20 ms before the instant so
     * the actuator's onset lands with the bloom. [alarm] marks a background firing.
     */
    fun exactInstant(context: Context, kind: String, alarm: Boolean = false) {
        if (!enabled(context)) return
        SkySound.touch()
        play(context, phrase(kind), alarm)
    }

    /** A press: one crisp click. */
    fun press(context: Context) {
        if (!enabled(context)) return
        SkySound.touch()
        play(context, DASHA_MINOR, false)
    }

    /** A selection: one soft tick. */
    fun select(context: Context) {
        if (!enabled(context)) return
        SkySound.touch()
        play(context, MOON, false)
    }

    private fun vibrator(context: Context): Vibrator? = runCatching {
        if (Build.VERSION.SDK_INT >= 31) {
            (context.getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as? VibratorManager)?.defaultVibrator
        } else {
            @Suppress("DEPRECATION")
            context.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
        }
    }.getOrNull()

    private fun play(context: Context, phrase: Phrase, alarm: Boolean) {
        val vib = vibrator(context)
        if (vib != null && runCatching { vib.hasVibrator() }.getOrDefault(false)) {
            val effect = effectFor(vib, phrase)
            if (effect != null) {
                runCatching { vibrate(vib, effect, alarm) }.onSuccess { return }
            }
        }
        // Last rung: the decor view's haptic feedback (works without the VIBRATE permission).
        val view = decorView(context) ?: return
        val constant = if (Build.VERSION.SDK_INT >= 30) phrase.viewConstant else phrase.viewConstantLegacy
        runCatching { view.performHapticFeedback(constant) }
    }

    private fun effectFor(vib: Vibrator, phrase: Phrase): VibrationEffect? {
        if (Build.VERSION.SDK_INT >= 30) {
            val composed = runCatching { composition(vib, phrase) }.getOrNull()
            if (composed != null) return composed
        }
        val amplitude = runCatching { vib.hasAmplitudeControl() }.getOrDefault(false)
        if (!amplitude && Build.VERSION.SDK_INT >= 29) {
            runCatching { VibrationEffect.createPredefined(phrase.predefined) }.getOrNull()?.let { return it }
        }
        return runCatching {
            if (phrase.timings.size == 2) {
                VibrationEffect.createOneShot(phrase.timings[1], if (amplitude) phrase.amplitudes[1] else VibrationEffect.DEFAULT_AMPLITUDE)
            } else if (amplitude) {
                VibrationEffect.createWaveform(phrase.timings, phrase.amplitudes, -1)
            } else {
                VibrationEffect.createWaveform(phrase.timings, -1)
            }
        }.getOrNull()
    }

    /** A composition of the phrase's primitives, or null when one is unsupported here. */
    private fun composition(vib: Vibrator, phrase: Phrase): VibrationEffect? {
        val ids = IntArray(phrase.prims.size) { substitute(phrase.prims[it].id) }
        if (!vib.areAllPrimitivesSupported(*ids)) return null
        val c = VibrationEffect.startComposition()
        for (i in phrase.prims.indices) {
            val p = phrase.prims[i]
            c.addPrimitive(ids[i], p.scale, p.delayMs)
        }
        return c.compose()
    }

    /** THUD and LOW_TICK exist from API 31; on 30 the nearest 30-level primitive stands in. */
    private fun substitute(id: Int): Int = when {
        Build.VERSION.SDK_INT >= 31 -> id
        id == P_THUD -> P_CLICK
        id == P_LOW_TICK -> P_TICK
        else -> id
    }

    private fun vibrate(vib: Vibrator, effect: VibrationEffect, alarm: Boolean) {
        if (!alarm) {
            vib.vibrate(effect)
            return
        }
        if (Build.VERSION.SDK_INT >= 33) {
            vib.vibrate(effect, VibrationAttributes.createForUsage(VibrationAttributes.USAGE_ALARM))
        } else {
            @Suppress("DEPRECATION")
            vib.vibrate(
                effect,
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_ALARM)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                    .build(),
            )
        }
    }

    private fun decorView(context: Context): View? {
        var c: Context? = context
        while (c is ContextWrapper) {
            if (c is Activity) return runCatching { c.window?.decorView }.getOrNull()
            c = c.baseContext
        }
        return null
    }
}
