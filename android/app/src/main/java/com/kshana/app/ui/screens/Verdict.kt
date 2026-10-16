package com.kshana.app.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.ui.theme.KshanaColors
import uniffi.kshana_ffi.AreaReading
import uniffi.kshana_ffi.PromiseRecord

/**
 * Both traditions' verdicts for one area, side by side, so neither hides the other:
 * KP's cusp sub-lord promise on the left, the Parashari assessment on the right.
 */
@Composable
internal fun VerdictPair(r: AreaReading, modifier: Modifier = Modifier) {
    val p = r.promise
    val parashariGood = r.parashariScore >= 0.5
    Row(modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
        Column(Modifier.weight(1f)) {
            Text("KP · SUB-LORD", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            Text(
                if (p.promised) "promised" else "not promised",
                fontFamily = FontFamily.Serif, fontSize = 22.sp,
                color = if (p.promised) KshanaColors.Gold else KshanaColors.TextSoft,
            )
            Text(signed(p.net), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky)
        }
        Column(Modifier.weight(1f)) {
            Text("PARASHARI", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            Text(
                if (parashariGood) "favourable" else "unfavourable",
                fontFamily = FontFamily.Serif, fontSize = 22.sp,
                color = if (parashariGood) KshanaColors.Gold else KshanaColors.TextSoft,
            )
            Text("%.2f · lord ${r.parashariLord}".format(r.parashariScore), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky)
        }
    }
}

/** The KP net with its two parts, so 0.00 is never shown without saying why. */
internal fun kpNetText(p: PromiseRecord): String =
    "net ${signed(p.net)} = fulfilling ${"%.2f".format(p.fulfillingWeight)} over houses ${p.group.joinToString(" ")}" +
        " − ½ × denying ${"%.2f".format(p.denyingWeight)} over houses ${p.negating.joinToString(" ")}"

/** One line that states whether the traditions agree and, if not, how they split. */
internal fun traditionsText(r: AreaReading): String {
    val p = r.promise
    if (r.traditionsAgree) return "The traditions agree."
    return if (p.promised) "The traditions disagree: KP promises it, Parashari scores it ${"%.2f".format(r.parashariScore)}."
    else "The traditions disagree: Parashari scores it ${"%.2f".format(r.parashariScore)}, but the KP sub-lord does not promise it."
}

/** Timing line that never claims convergence for a matter the KP rule denies. */
internal fun timingText(r: AreaReading, levels: Int): String {
    val n = r.agreeingLevels.toInt()
    val base = "$n of $levels dasha levels touch the houses"
    return when {
        n > 0 && !r.promise.promised -> "$base · the period is ripe, but the KP sub-lord does not promise it"
        n == 0 -> "$base · no running period touches the houses"
        else -> base
    }
}

internal fun signed(x: Double): String = (if (x < 0) "−" else "+") + "%.2f".format(kotlin.math.abs(x))
