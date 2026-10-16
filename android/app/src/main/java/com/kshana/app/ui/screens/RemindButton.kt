package com.kshana.app.ui.screens

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.os.Build
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.kshana.app.notify.Reminders
import com.kshana.app.ui.theme.KshanaColors

private const val REQUEST_POST_NOTIFICATIONS = 41

/**
 * A small quiet toggle: "Remind me" sets an exact alarm for `atMillis`; "Reminder set"
 * (gold) clears it. On Android 13+ the first tap also asks for the notification permission.
 * Nothing is drawn once the moment has passed.
 */
@Composable
internal fun RemindButton(key: String, atMillis: Long, title: String, text: String, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val id = remember(key) { Reminders.stableId(key) }
    var set by remember(key) { mutableStateOf(Reminders.isScheduled(context, id)) }
    if (atMillis <= System.currentTimeMillis()) return

    val shape = RoundedCornerShape(8.dp)
    Box(
        modifier = modifier
            .height(36.dp)
            .clip(shape)
            .border(1.dp, if (set) KshanaColors.Gold else KshanaColors.Line, shape)
            .clickable {
                if (set) {
                    Reminders.cancel(context, id)
                    set = false
                } else {
                    if (Build.VERSION.SDK_INT >= 33 && !Reminders.canPostNotifications(context)) requestNotificationPermission(context)
                    Reminders.schedule(context, id, atMillis, title, text)
                    set = true
                }
            }
            .padding(horizontal = 12.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            if (set) "REMINDER SET" else "REMIND ME",
            style = MaterialTheme.typography.labelSmall,
            color = if (set) KshanaColors.Gold else KshanaColors.Muted,
        )
    }
}

/** Ask for POST_NOTIFICATIONS through the hosting activity, if one can be found. */
private fun requestNotificationPermission(context: Context) {
    if (Build.VERSION.SDK_INT < 33) return
    val activity = activityOf(context) ?: return
    activity.requestPermissions(arrayOf(android.Manifest.permission.POST_NOTIFICATIONS), REQUEST_POST_NOTIFICATIONS)
}

private fun activityOf(context: Context): Activity? {
    var c: Context? = context
    while (c != null) {
        if (c is Activity) return c
        c = (c as? ContextWrapper)?.baseContext
    }
    return null
}
