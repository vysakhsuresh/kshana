package com.kshana.app.ui.screens

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.kshana.app.data.Backup
import com.kshana.app.data.Faith
import com.kshana.app.data.RemedyPlans
import com.kshana.app.ui.theme.KshanaColors
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

@Composable
internal fun KSwitch(checked: Boolean, onChange: (Boolean) -> Unit) {
    Switch(
        checked = checked, onCheckedChange = onChange,
        colors = SwitchDefaults.colors(
            checkedThumbColor = KshanaColors.Night, checkedTrackColor = KshanaColors.Gold,
            uncheckedThumbColor = KshanaColors.Muted, uncheckedTrackColor = KshanaColors.Surface, uncheckedBorderColor = KshanaColors.Line,
        ),
    )
}

/** Your data: a file you keep, a file you restore, and the automatic copy in Downloads. */
@Composable
internal fun DataSettings() {
    val context = LocalContext.current
    var note by remember { mutableStateOf<String?>(null) }
    var auto by remember { mutableStateOf(Backup.autoOn(context)) }
    val save = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/json")) { uri ->
        if (uri != null) note = if (Backup.write(context, uri)) "Saved. Keep the file somewhere safe: Drive, email to yourself, or a pen drive." else "Could not save the file."
    }
    val open = rememberLauncherForActivityResult(com.kshana.app.data.OpenBackup()) { uri ->
        if (uri != null) note = Backup.read(context, uri)?.let { "Restored${if (it.isNotBlank()) " for $it" else ""}. Close and open the app to see everything." } ?: "That file is not a Kshana backup."
    }
    SectionLabel("Your data")
    Text(
        "Everything you entered: your birth, your people, your journal, your record and your remedies. It never leaves the phone unless you send the file yourself.",
        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
    )
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        QuietButton("SAVE A BACKUP FILE", onClick = { save.launch(Backup.FILE_NAME) }, modifier = Modifier.weight(1f))
        QuietButton("RESTORE", onClick = { open.launch(Unit) }, modifier = Modifier.weight(1f))
    }
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text("Keep a copy in Downloads", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
            val last = Backup.lastSaved(context)
            Text(
                if (last > 0) "Last copy ${Instant.ofEpochMilli(last).atZone(ZoneId.systemDefault()).format(DateTimeFormatter.ofPattern("d MMM, HH:mm"))} · Downloads › Kshana" else "So an uninstall or a cleared phone never loses your data.",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
        }
        KSwitch(auto) { auto = it; Backup.setAuto(context, it); if (it) Backup.autoSave(context) }
    }
    note?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky) }
}

/** How remedies are offered: deeds for everyone first, faith practices as the person chooses. */
@Composable
internal fun FaithSettings() {
    val context = LocalContext.current
    var faith by remember { mutableStateOf(Faith.get(context)) }
    SectionLabel("Remedies for you")
    Text(
        "Every remedy starts with deeds anyone can do. Choose what else to show: prayer in your own faith, the Hindu tradition (poojas, mantras, temples), or deeds only.",
        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
    )
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for ((k, label) in Faith.OPTIONS) Chip(label, selected = faith == k, onClick = { faith = k; Faith.set(context, k) })
    }
}

/** Where "give food" and "give money" remedies send the gift. */
@Composable
internal fun GivingSettings() {
    val context = LocalContext.current
    var id by remember { mutableStateOf(RemedyPlans.upi(context)) }
    var name by remember { mutableStateOf(RemedyPlans.upiName(context)) }
    SectionLabel("Giving")
    Text(
        "When a remedy asks you to give, one tap opens your UPI app to pay the cause you choose. Enter its UPI ID (for example a temple, mosque, church, gurdwara, orphanage or food bank you trust). Without one, a food charity's page opens.",
        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
    )
    val colors = OutlinedTextFieldDefaults.colors(focusedBorderColor = KshanaColors.Gold, unfocusedBorderColor = KshanaColors.Line, focusedLabelColor = KshanaColors.Gold)
    OutlinedTextField(value = name, onValueChange = { name = it; RemedyPlans.setUpi(context, id, it) }, label = { Text("Name of the cause") }, singleLine = true, colors = colors, modifier = Modifier.fillMaxWidth())
    OutlinedTextField(value = id, onValueChange = { id = it; RemedyPlans.setUpi(context, it, name) }, label = { Text("Its UPI ID (name@bank)") }, singleLine = true, colors = colors, modifier = Modifier.fillMaxWidth())
}

/** If the app ever crashed, the report sits here, on the phone, for the person to send or delete. */
@Composable
internal fun CrashSettings() {
    val context = LocalContext.current
    var last by remember { mutableStateOf(com.kshana.app.system.CrashLog.last(context)) }
    if (last == null) return
    SectionLabel("Problem report")
    Text("The app stopped unexpectedly once. The report below stays on this phone; send it to the developer if you like, or delete it.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        QuietButton("SEND REPORT", onClick = { com.kshana.app.system.CrashLog.shareIntent(context)?.let { runCatching { context.startActivity(it) } } }, modifier = Modifier.weight(1f))
        QuietButton("DELETE", onClick = { com.kshana.app.system.CrashLog.clear(context); last = null }, modifier = Modifier.weight(1f))
    }
}
