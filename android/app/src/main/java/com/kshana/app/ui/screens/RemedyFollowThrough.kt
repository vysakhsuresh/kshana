package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.FollowThrough
import com.kshana.app.data.TakenRemedy
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import uniffi.kshana_ffi.RemedyScheduleRecord
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

private val SHORT_DATE = DateTimeFormatter.ofPattern("d MMM")

private fun shortDate(millis: Long): String = Instant.ofEpochMilli(millis).atZone(ZoneId.systemDefault()).format(SHORT_DATE)

/** The end of what the question asks about: the stretch's end or the last day, whichever came first. */
private fun askedUntil(t: TakenRemedy): Long {
    val stretch = t.stretchEndMillis?.takeIf { it > t.startMillis + 86_400_000L }
    return minOf(t.untilMillis, stretch ?: t.untilMillis, System.currentTimeMillis())
}

/** What Kshana says back, humanely, to each answer. Never a claim that the remedy did it. */
private fun replyTo(answer: String): String = when (answer) {
    FollowThrough.EASED -> "Good. Things eased while you kept it. Keep the parts that helped; they are yours now."
    FollowThrough.SAME -> "Thank you for saying so. Holding steady through a hard stretch is something. You can keep it going, or try another way below."
    FollowThrough.HARDER -> "Thank you for telling it straight. A harder stretch is not your failing, and it does pass. Lean on the people near you, keep one small good deed going, and look at what else helps."
    else -> "That is all right; life gets full. If you want, begin again at the next good hour. Even one small step counts."
}

/**
 * The gentle question once a remedy's stretch or its days are over: "Did things ease?",
 * with Eased / Same / Harder / Didn't do it, and "ask me later". Draws nothing when no
 * check-in is due. Place it on the Now tab; [onOpen] opens the remedy page.
 */
@Composable
fun RemedyCheckInCard(profile: BirthProfile, onOpen: () -> Unit) {
    var version by remember { mutableStateOf(0) }
    CheckInBlock(profile.name, version, onChange = { version++ }, onOpen = onOpen)
}

/** The first due check-in, or the reply to the one just answered. */
@Composable
private fun CheckInBlock(profileName: String, version: Int, onChange: () -> Unit, onOpen: (() -> Unit)?) {
    val context = LocalContext.current
    val due = remember(profileName, version) { FollowThrough.due(context, profileName) }
    var replied by remember(profileName) { mutableStateOf<Pair<String, String>?>(null) }
    val t = due.firstOrNull()
    val reply = replied
    if (t == null && reply == null) return
    Column(Modifier.fillMaxWidth().glass(16.dp).padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("A GENTLE CHECK-IN", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        if (reply != null) {
            Text(reply.first, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            Text(replyTo(reply.second), fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                if (onOpen != null) QuietButton(if (reply.second == FollowThrough.EASED) "SEE YOUR REMEDIES" else "SEE WHAT HELPS", onClick = onOpen)
                Text(
                    if (t != null) "NEXT ›" else "CLOSE",
                    style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted,
                    modifier = Modifier.clickable { replied = null }.padding(12.dp),
                )
            }
            return@Column
        }
        t ?: return@Column
        Text(
            "Did things ease between ${shortDate(t.startMillis)} and ${shortDate(askedUntil(t))}, while you kept “${t.title}”?",
            fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 25.sp, color = KshanaColors.Text,
        )
        if (t.forStretch.isNotBlank()) Text(t.forStretch, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((label, answer) in listOf("EASED" to FollowThrough.EASED, "SAME" to FollowThrough.SAME, "HARDER" to FollowThrough.HARDER, "DIDN'T DO IT" to FollowThrough.SKIPPED)) {
                Chip(label, selected = false, onClick = {
                    FollowThrough.answer(context, profileName, t.remedyId, answer)
                    replied = "“${t.title}”" to answer
                    onChange()
                })
            }
        }
        Text(
            "There is no wrong answer; it only keeps your own record honest. ASK ME LATER",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            modifier = Modifier.clickable { FollowThrough.later(context, profileName, t.remedyId); onChange() },
        )
    }
}

/**
 * The remedy page's record: any check-in that is due, then the honest tally of what was
 * kept and what the person saw ("You kept 3 remedies; 2 eased, 1 the same").
 */
@Composable
internal fun RemedyRecordCard(profileName: String, version: Int, onChange: () -> Unit) {
    val context = LocalContext.current
    CheckInBlock(profileName, version, onChange, onOpen = null)
    val all = remember(profileName, version) { FollowThrough.all(context, profileName) }
    val words = FollowThrough.recordWords(FollowThrough.record(all)) ?: return
    val open = all.count { it.answer == null }
    Column(Modifier.fillMaxWidth().glass(16.dp).padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text("YOUR RECORD", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(words, fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text)
        if (open > 0) Text("$open still under way.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
        Text(
            "This is what you saw while keeping them, in your own words. It is not proof that a remedy caused it, and the record is only on your phone.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
    }
}

/**
 * When to begin a remedy, for how long, and for which stretch: the exact first hour with
 * a reminder at it, and "take it on", which starts the record and the later check-in.
 */
@Composable
internal fun RemedyStartBlock(profileName: String, s: RemedyScheduleRecord, onTaken: () -> Unit) {
    val context = LocalContext.current
    var taken by remember(s.remedyId) { mutableStateOf(FollowThrough.open(context, profileName, s.remedyId) != null) }
    var why by remember(s.remedyId) { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().padding(top = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("BEGIN", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(s.startWords, fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
        Text("Keep it ${s.keepWords}, ${s.untilWords}.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
        Text(s.keepBasisWords.replaceFirstChar { it.uppercase() } + ".", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        Text(s.forStretchWords, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
        Text(
            if (why) "WHY THIS HOUR −" else "WHY THIS HOUR +",
            style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
            modifier = Modifier.clickable { why = !why }.padding(vertical = 4.dp),
        )
        if (why) Text(s.whyNow, style = MaterialTheme.typography.bodySmall, lineHeight = 18.sp, color = KshanaColors.TextSoft)
        Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            QuietButton(if (taken) "TAKEN ON ✓" else "TAKE IT ON", onClick = {
                if (taken) {
                    FollowThrough.drop(context, profileName, s.remedyId)
                    taken = false
                } else {
                    FollowThrough.take(context, profileName, s)
                    taken = true
                }
                onTaken()
            })
            RemindButton(key = "remedy:${s.remedyId}@${s.startMs}", atMillis = s.startMs, title = s.title, text = s.reminderText)
        }
    }
}
