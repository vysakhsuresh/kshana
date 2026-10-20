package com.kshana.app.ui.sky

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.delay

/*
 * The bell: one exact instant rendered as bloom + chime + thud, phase-locked from one clock.
 *
 * The three channels are armed from the same target, the instant's epoch millisecond:
 * visual onset on the first frame (withFrameNanos) whose wall clock has passed the
 * instant, never a frame early; the haptic phrase and the chime 20 ms before it, the
 * typical actuator onset, so the thud lands with the bloom (audio adds its own 20–45 ms of
 * output latency, which is accepted and stated). The bloom is a gold ring that expands
 * over 1.2 s and fades while the words come up in the serif; the words then hold for a
 * few seconds. A bell that the device would witness more than 3 s late (the app was in the
 * background and the frame clock paused) is skipped rather than rung late: the bell only
 * rings at the instant. Under the motion contract the ring is a single static frame.
 */

/** An upcoming instant for [BellHost]: when, which haptic phrase, and the words to show. */
data class BellInstant(val atEpochMillis: Long, val kind: String, val words: String)

private const val BLOOM_MILLIS = 1200L
private const val HAPTIC_LEAD_MILLIS = 20L
private const val MISSED_AFTER_MILLIS = 3000L

/**
 * Ring the bell when the device clock passes [atEpochMillis]: a gold bloom, the words in
 * serif, the haptic phrase for [kind] and, if sound is on, the chime. The words hold for
 * [holdMillis] after the bloom, then [onDone] is called (also when the instant was missed).
 */
@Composable
fun ExactMomentBell(
    atEpochMillis: Long,
    kind: String,
    words: String,
    modifier: Modifier = Modifier,
    holdMillis: Long = 4000L,
    onDone: () -> Unit = {},
) {
    val context = LocalContext.current
    val reduced by rememberUpdatedState(Motion.rememberReduced())
    val fonts = rememberKshanaFonts()
    val density = LocalDensity.current
    // −1 waiting · 0..1 blooming · 2 words held · 3 done. Read only in the draw phase.
    var progress by remember(atEpochMillis, kind) { mutableFloatStateOf(-1f) }
    val currentOnDone by rememberUpdatedState(onDone)

    LaunchedEffect(atEpochMillis, kind) {
        if (System.currentTimeMillis() - atEpochMillis > MISSED_AFTER_MILLIS) {
            progress = 3f
            currentOnDone()
            return@LaunchedEffect
        }
        var cued = false
        while (true) {
            withFrameNanos { }
            val remaining = atEpochMillis - System.currentTimeMillis()
            if (!cued && remaining <= HAPTIC_LEAD_MILLIS) {
                cued = true
                Haptics.exactInstant(context, kind)
                SkySound.chime(kind, context)
            }
            if (remaining <= 0L) break
            // Far away: sleep in steps so the frame loop does not spin for hours.
            if (remaining > 1500L) delay(minOf(remaining - 1200L, 1000L))
        }
        if (System.currentTimeMillis() - atEpochMillis > MISSED_AFTER_MILLIS) {
            progress = 3f
            currentOnDone()
            return@LaunchedEffect
        }
        val start = withFrameNanos { it }
        if (reduced) {
            progress = 0.45f
            delay(BLOOM_MILLIS)
        } else {
            while (true) {
                val t = withFrameNanos { it }
                val p = (t - start) / (BLOOM_MILLIS * 1_000_000f)
                if (p >= 1f) break
                progress = p
            }
        }
        progress = 2f
        delay(holdMillis)
        progress = 3f
        currentOnDone()
    }

    val stroke = remember(density) { Stroke(with(density) { 1.5.dp.toPx() }) }
    val minRadius = with(density) { 24.dp.toPx() }
    val growRadius = with(density) { 120.dp.toPx() }

    Box(modifier, contentAlignment = Alignment.Center) {
        Canvas(Modifier.matchParentSize()) {
            val p = progress
            if (p >= 0f && p <= 1f) {
                val k = 1f - p
                val eased = 1f - k * k * k
                val r = minRadius + growRadius * eased
                drawCircle(KshanaColors.Gold, r, center, alpha = k * 0.9f, style = stroke)
                drawCircle(KshanaColors.Gold, r * 0.55f, center, alpha = k * 0.22f)
            }
        }
        Text(
            words,
            fontFamily = fonts.serif,
            fontSize = 20.sp,
            lineHeight = 26.sp,
            color = KshanaColors.Text,
            textAlign = TextAlign.Center,
            modifier = Modifier
                .padding(horizontal = 24.dp)
                .graphicsLayer {
                    val p = progress
                    alpha = when {
                        p < 0f -> 0f
                        p <= 1f -> p
                        p < 3f -> 1f
                        else -> 0f
                    }
                },
        )
    }
}

/**
 * Watch a list of upcoming instants and ring [ExactMomentBell] for each in turn, earliest
 * first. Instants more than 3 s in the past are skipped. For the Today and River screens.
 */
@Composable
fun BellHost(instants: List<BellInstant>, modifier: Modifier = Modifier, holdMillis: Long = 4000L) {
    var done by remember { mutableStateOf<Set<BellInstant>>(emptySet()) }
    val now = System.currentTimeMillis()
    val next = instants
        .filter { it !in done && it.atEpochMillis + MISSED_AFTER_MILLIS > now }
        .minByOrNull { it.atEpochMillis }
    if (next != null) {
        key(next) {
            ExactMomentBell(
                next.atEpochMillis, next.kind, next.words, modifier, holdMillis,
                onDone = { done = (done + next).filterTo(HashSet()) { it in instants } },
            )
        }
    }
}
