package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import com.kshana.app.ui.theme.Grotesk
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import uniffi.kshana_ffi.ExactInstant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/* Shared, package-private helpers for the screens. `signName` and `dms` live in NowScreen.kt. */

internal val PLANET_GLYPHS: Map<String, String> = mapOf(
    "Sun" to "☉", "Moon" to "☽", "Mercury" to "☿", "Venus" to "♀", "Mars" to "♂",
    "Jupiter" to "♃", "Saturn" to "♄", "Uranus" to "♅", "Neptune" to "♆", "Pluto" to "♇",
    "Rahu" to "☊", "Ketu" to "☋",
)

internal val PLANET_ABBR: Map<String, String> = mapOf(
    "Sun" to "Su", "Moon" to "Mo", "Mars" to "Ma", "Mercury" to "Me", "Jupiter" to "Ju",
    "Venus" to "Ve", "Saturn" to "Sa", "Rahu" to "Ra", "Ketu" to "Ke",
    "Uranus" to "Ur", "Neptune" to "Ne", "Pluto" to "Pl",
)

internal val SIGN_SHORT = listOf("Ari", "Tau", "Gem", "Can", "Leo", "Vir", "Lib", "Sco", "Sag", "Cap", "Aqu", "Pis")

internal val DATE_TIME = DateTimeFormatter.ofPattern("d MMM uuuu HH:mm")
internal val DATE_TIME_S = DateTimeFormatter.ofPattern("d MMM uuuu HH:mm:ss")
internal val DATE_ONLY = DateTimeFormatter.ofPattern("d MMM uuuu")
internal val TIME_ONLY = DateTimeFormatter.ofPattern("HH:mm:ss")

internal fun norm360(x: Double): Double = (x % 360.0 + 360.0) % 360.0

/** Degrees and minutes within the sign, e.g. 17°42′. */
internal fun degMin(lon: Double): String {
    val inSign = norm360(lon) % 30.0
    val d = inSign.toInt()
    val m = ((inSign - d) * 60.0).toInt()
    return "%d°%02d′".format(d, m)
}

internal fun signIndex(lon: Double): Int = (norm360(lon) / 30.0).toInt() % 12

/** Seconds since the TAI epoch as a double, for progress bars and ordering. */
internal fun ExactInstant.seconds(): Double = taiSeconds.toDouble() + nanos.toDouble() / 1e9

/** ".nnn nnn" microseconds within the second. */
internal fun microText(e: ExactInstant): String {
    val micro = (e.nanos.toLong() / 1_000).toString().padStart(6, '0')
    return ".${micro.substring(0, 3)} ${micro.substring(3)}"
}

internal fun localText(e: ExactInstant, pattern: DateTimeFormatter = DATE_TIME, zone: ZoneId = ZoneId.systemDefault()): String =
    Instants.toLocal(e, zone).format(pattern)

/** Fraction of the way from start to end, clamped. */
internal fun progress(start: ExactInstant, end: ExactInstant, at: ExactInstant): Float {
    val span = end.seconds() - start.seconds()
    if (span <= 0.0) return 0f
    return ((at.seconds() - start.seconds()) / span).coerceIn(0.0, 1.0).toFloat()
}

/** A factor string as a sentence: capitalised, ending in a full stop. Nothing is added. */
internal fun sentence(factor: String): String {
    val t = factor.trim()
    if (t.isEmpty()) return t
    val s = t.replaceFirstChar { it.uppercaseChar() }
    return if (s.endsWith(".") || s.endsWith("!") || s.endsWith("?")) s else "$s."
}

@Composable
internal fun SectionLabel(text: String, modifier: Modifier = Modifier) {
    Text(text.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = modifier.padding(top = 18.dp, bottom = 6.dp))
}

@Composable
internal fun ErrorLine(error: String?) {
    if (error != null) {
        Text(error, color = KshanaColors.Sky, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(vertical = 6.dp))
    }
}

/** A 44 dp pill chip in the night palette: the one filled selection on a screen (a topic). */
@Composable
internal fun Chip(label: String, selected: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(22.dp)
    Box(
        modifier = modifier
            .height(44.dp)
            .clip(shape)
            .background(if (selected) KshanaColors.Gold else KshanaColors.Surface)
            .border(1.dp, if (selected) KshanaColors.Gold else KshanaColors.Line, shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            label,
            fontFamily = Grotesk,
            fontWeight = FontWeight.Medium,
            fontSize = 12.sp,
            letterSpacing = 1.sp,
            color = if (selected) KshanaColors.Night else KshanaColors.TextSoft,
        )
    }
}

/**
 * A quiet pill for a value the person can change (a date, a time): outlined, never
 * filled, so the gold fill stays for the topic chip and the one call to action.
 */
@Composable
internal fun ValuePill(label: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(22.dp)
    Box(
        modifier = modifier
            .height(44.dp)
            .clip(shape)
            .background(KshanaColors.Surface)
            .border(1.dp, KshanaColors.Gold.copy(alpha = 0.7f), shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(label, fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = KshanaColors.Gold)
    }
}

/**
 * One option among several (a question, a purpose): a surface box with a small ring on
 * the left. Selected: gold ring filled, gold border, text in the UI sans. Never filled,
 * so a screen keeps one gold block, the button.
 */
@Composable
internal fun OptionBox(label: String, selected: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(12.dp)
    Row(
        modifier = modifier
            .clip(shape)
            .background(KshanaColors.Surface)
            .border(1.dp, if (selected) KshanaColors.Gold else KshanaColors.LineSoft, shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            Modifier
                .size(10.dp)
                .clip(CircleShape)
                .border(1.5.dp, if (selected) KshanaColors.Gold else KshanaColors.Muted, CircleShape)
                .background(if (selected) KshanaColors.Gold else Color.Transparent),
        )
        Spacer(Modifier.width(10.dp))
        Text(
            label,
            fontFamily = Grotesk,
            fontWeight = if (selected) FontWeight.Medium else FontWeight.Normal,
            fontSize = 14.sp,
            lineHeight = 18.sp,
            color = if (selected) KshanaColors.Gold else KshanaColors.Text,
        )
    }
}

/** The one gold call-to-action: full width, 52 dp, serif capitals. */
@Composable
internal fun GoldButton(label: String, enabled: Boolean = true, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(8.dp)
    Box(
        modifier = modifier
            .height(52.dp)
            .clip(shape)
            .background(if (enabled) KshanaColors.Gold else KshanaColors.Line)
            .clickable(enabled = enabled, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Text(label, fontFamily = FontFamily.Serif, fontSize = 16.sp, letterSpacing = 4.sp, color = if (enabled) KshanaColors.Night else KshanaColors.Muted)
    }
}

/** A quiet secondary action: outlined, 44 dp. */
@Composable
internal fun QuietButton(label: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(8.dp)
    Box(
        modifier = modifier
            .height(44.dp)
            .clip(shape)
            .border(1.dp, KshanaColors.Line, shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(label, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
    }
}

/** A label/value pair in a row: label muted sans, value mono sky. */
@Composable
internal fun MonoRow(label: String, value: String, valueColor: androidx.compose.ui.graphics.Color = KshanaColors.Sky) {
    Row(Modifier.padding(vertical = 3.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(label, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
        Text(value, fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = valueColor)
    }
}
