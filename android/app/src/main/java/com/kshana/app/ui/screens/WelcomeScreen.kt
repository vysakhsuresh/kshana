package com.kshana.app.ui.screens

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.Backup
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.StarWheel

/**
 * The first screen on a fresh install: what the app is, in three lines, and the two
 * ways in: your birth details, or the backup file from before.
 */
@Composable
fun WelcomeScreen(onStart: () -> Unit, onRestored: () -> Unit) {
    val context = LocalContext.current
    var note by remember { mutableStateOf<String?>(null) }
    val open = rememberLauncherForActivityResult(com.kshana.app.data.OpenBackup()) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        val name = Backup.read(context, uri)
        if (name == null) note = "That file is not a Kshana backup. It is called ${Backup.FILE_NAME} and is usually in Downloads › Kshana."
        else onRestored()
    }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 28.dp).padding(top = 72.dp, bottom = 40.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        StarWheel(size = 72.dp, modifier = Modifier.height(72.dp).fillMaxWidth())
        Text("K S H A N A", fontFamily = FontFamily.Serif, fontSize = 22.sp, letterSpacing = 4.sp, color = KshanaColors.Gold)
        Text("Your time, read plainly.", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "From the moment you were born, Kshana works out what each day, week, month and year holds for your love, work, money, health and family: when it is good, when to go gently, and what helps.",
            fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.TextSoft,
        )
        Text(
            "No account. No internet needed. Nothing you enter leaves this phone. It is guidance, never a guarantee, and it shows you openly how often it has been right for you.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        Spacer(Modifier.height(10.dp))
        GoldButton("BEGIN WITH MY BIRTH DETAILS", onClick = onStart, modifier = Modifier.fillMaxWidth())
        QuietButton("RESTORE FROM MY BACKUP FILE", onClick = { open.launch(Unit) }, modifier = Modifier.fillMaxWidth())
        Text("Used Kshana before? Your backup is kept in Downloads › Kshana › ${Backup.FILE_NAME}.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        note?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Ember) }
    }
}
