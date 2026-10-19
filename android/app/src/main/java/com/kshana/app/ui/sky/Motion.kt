package com.kshana.app.ui.sky

import android.animation.ValueAnimator
import android.content.ComponentCallbacks2
import android.content.Context
import android.content.res.Configuration
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.os.PowerManager
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import com.kshana.app.data.ProfileStore
import kotlinx.coroutines.delay
import java.util.concurrent.CopyOnWriteArrayList

/*
 * The motion contract for the sky package.
 *
 * Every animation in `ui.sky` asks [Motion.reduced] first and falls back to a static frame
 * when it is true. It is true when the system has animators off (the accessibility
 * "Remove animations" toggle or the developer animator scale at 0, which is what
 * `ValueAnimator.areAnimatorsEnabled()` reports), when the user has switched on
 * "Still sky" in the app (prefs key `ui.stillSky`), or when Battery Saver is on. Nothing in
 * Kshana depends on motion: every breathing ring also prints its number, every bloom also
 * writes the instant.
 *
 * Budgets: breathing at 0.1–0.25 Hz, tilt parallax of at most 8 dp for the stars and 4 dp
 * for the ring, crossfades of 150 ms instead of slides when reduced.
 */
object Motion {
    const val PREF_STILL_SKY = "ui.stillSky"

    /** Breathing period: 5 s = 0.2 Hz, inside the 0.1–0.25 Hz band. */
    const val BREATH_PERIOD_MS = 5000

    /** One twinkle cycle; sin is continuous across the wrap. */
    const val TWINKLE_PERIOD_MS = 6000

    /** The star layer may shift this much under tilt; the ring half of it; text none. */
    const val PARALLAX_STARS_DP = 8f
    const val PARALLAX_RING_DP = 4f

    /** When reduced, state changes crossfade over this instead of moving. */
    const val CROSSFADE_MS = 150

    /** The in-app "Still sky" switch (default off). */
    fun stillSky(context: Context): Boolean =
        runCatching { ProfileStore.prefs(context).getBoolean(PREF_STILL_SKY, false) }.getOrDefault(false)

    fun setStillSky(context: Context, still: Boolean) {
        runCatching { ProfileStore.prefs(context).edit().putBoolean(PREF_STILL_SKY, still).apply() }
    }

    /** False when the animator duration scale is 0 or Battery Saver disabled animators. */
    fun animatorsEnabled(): Boolean = runCatching { ValueAnimator.areAnimatorsEnabled() }.getOrDefault(true)

    fun powerSave(context: Context): Boolean = runCatching {
        (context.getSystemService(Context.POWER_SERVICE) as? PowerManager)?.isPowerSaveMode == true
    }.getOrDefault(false)

    /** The contract: animate only when this is false. */
    fun reduced(context: Context): Boolean = !animatorsEnabled() || stillSky(context) || powerSave(context)

    /**
     * [reduced] as state, re-read every [pollMillis] so a Battery Saver or settings change
     * stills the sky within two seconds without a receiver.
     */
    @Composable
    fun rememberReduced(pollMillis: Long = 2000L): Boolean {
        val context = LocalContext.current
        var reduced by remember(context) { mutableStateOf(reduced(context)) }
        LaunchedEffect(context, pollMillis) {
            while (true) {
                delay(pollMillis)
                val now = reduced(context)
                if (now != reduced) reduced = now
            }
        }
        return reduced
    }
}

/**
 * Whether the app's UI is on screen, without a lifecycle library: the process is told
 * `TRIM_MEMORY_UI_HIDDEN` when its last window leaves the screen, and the Compose frame
 * clock (which pauses while the activity is stopped) tells us when frames flow again.
 * Sensors and sound hang off this so they never run in the background.
 */
object Foreground {
    private val hiddenState = mutableStateOf(false)
    private val onHidden = CopyOnWriteArrayList<() -> Unit>()
    @Volatile private var registered = false

    /** True from the moment the UI is hidden until the next frame is drawn. */
    val hidden: State<Boolean> get() = hiddenState

    /** Register the process callback once; later calls are no-ops. */
    fun register(context: Context) {
        if (registered) return
        synchronized(this) {
            if (registered) return
            registered = true
            runCatching {
                context.applicationContext.registerComponentCallbacks(object : ComponentCallbacks2 {
                    override fun onTrimMemory(level: Int) {
                        if (level >= ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN) markHidden()
                    }
                    override fun onConfigurationChanged(newConfig: Configuration) {}
                    @Deprecated("Deprecated in Java")
                    override fun onLowMemory() {}
                })
            }
        }
    }

    /** Called when the UI goes away; used by the sound engine to stop. */
    fun addOnHidden(listener: () -> Unit) {
        if (!onHidden.contains(listener)) onHidden.add(listener)
    }

    private fun markHidden() {
        hiddenState.value = true
        for (l in onHidden) runCatching { l() }
    }

    /** A frame was drawn: the UI is visible again. Called from the sky's frame loop. */
    fun sawFrame() {
        if (hiddenState.value) hiddenState.value = false
    }
}

/** The current parallax shift in dp, written by the accelerometer and read in the draw phase. */
class TiltState {
    var x by mutableFloatStateOf(0f)
    var y by mutableFloatStateOf(0f)
}

/**
 * Tilt parallax from the accelerometer: the gravity vector is low-pass filtered twice, a
 * fast follower (the phone as held now) and a slow one (the resting pose over ~5 s); the
 * difference, capped at [maxDp], is the shift. So tilting the phone slides the stars by
 * up to 8 dp and they settle back as the hand settles. Registered at SENSOR_DELAY_UI only
 * while [enabled] and the UI is visible; unregistered on dispose and when the UI hides.
 * Disabled entirely under the motion contract (pass `enabled = !reduced`).
 */
@Composable
fun rememberTilt(enabled: Boolean, maxDp: Float = Motion.PARALLAX_STARS_DP): TiltState {
    val context = LocalContext.current
    val state = remember { TiltState() }
    val hidden by Foreground.hidden
    val active = enabled && !hidden
    DisposableEffect(active, maxDp, context) {
        val manager = if (active) context.getSystemService(Context.SENSOR_SERVICE) as? SensorManager else null
        val sensor = manager?.getDefaultSensor(Sensor.TYPE_ACCELEROMETER)
        if (manager == null || sensor == null) {
            state.x = 0f
            state.y = 0f
            onDispose { }
        } else {
            val listener = object : SensorEventListener {
                private var fx = 0f
                private var fy = 0f
                private var sx = 0f
                private var sy = 0f
                private var primed = false
                private val dpPerMs2 = maxDp / 2.5f // 2.5 m/s² (about 15° of tilt) reaches the cap

                override fun onSensorChanged(event: SensorEvent) {
                    val gx = event.values[0]
                    val gy = event.values[1]
                    if (!primed) {
                        fx = gx; fy = gy; sx = gx; sy = gy
                        primed = true
                        return
                    }
                    fx += 0.15f * (gx - fx)
                    fy += 0.15f * (gy - fy)
                    sx += 0.01f * (fx - sx)
                    sy += 0.01f * (fy - sy)
                    state.x = ((sx - fx) * dpPerMs2).coerceIn(-maxDp, maxDp)
                    state.y = ((fy - sy) * dpPerMs2).coerceIn(-maxDp, maxDp)
                }

                override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) {}
            }
            runCatching { manager.registerListener(listener, sensor, SensorManager.SENSOR_DELAY_UI) }
            onDispose {
                runCatching { manager.unregisterListener(listener) }
                state.x = 0f
                state.y = 0f
            }
        }
    }
    return state
}
