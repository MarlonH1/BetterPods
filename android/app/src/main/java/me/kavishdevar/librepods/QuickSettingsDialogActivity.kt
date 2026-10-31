/*
    LibrePods - AirPods liberated from Apple’s ecosystem
    Copyright (C) 2025 LibrePods contributors

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

@file:OptIn(ExperimentalEncodingApi::class, androidx.compose.material3.ExperimentalMaterial3Api::class)

package me.kavishdevar.librepods

import android.content.Intent
import android.os.Bundle
import android.view.Gravity
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.scaleIn
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedButton
import androidx.compose.runtime.mutableStateOf
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import me.kavishdevar.librepods.bluetooth.AACPManager
import me.kavishdevar.librepods.bluetooth.AACPManager.Companion.ControlCommandIdentifiers
import me.kavishdevar.librepods.bluetooth.BluetoothConnectionManager
import me.kavishdevar.librepods.data.AirPodsMax
import me.kavishdevar.librepods.data.Battery
import me.kavishdevar.librepods.data.BatteryComponent
import me.kavishdevar.librepods.data.BatteryStatus
import me.kavishdevar.librepods.data.Capability
import me.kavishdevar.librepods.presentation.components.ListeningModeGroup
import me.kavishdevar.librepods.presentation.components.listeningModeOptions
import me.kavishdevar.librepods.presentation.theme.LibrePodsTheme
import me.kavishdevar.librepods.presentation.widgets.WidgetRenderer
import me.kavishdevar.librepods.services.CompanionSessions
import me.kavishdevar.librepods.services.ServiceManager
import kotlin.io.encoding.ExperimentalEncodingApi

/**
 * Opened from the Quick Settings tile. Styled after SystemUI's own tile dialogs (Bluetooth,
 * Internet): centered, blurred backdrop, icon + title header, device rows and Settings/Done.
 */
class QuickSettingsDialogActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.setGravity(Gravity.CENTER)
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.S) {
            window.addFlags(android.view.WindowManager.LayoutParams.FLAG_BLUR_BEHIND)
            window.attributes = window.attributes.apply { blurBehindRadius = 80 }
        }
        window.setDimAmount(0.4f)
        setContent {
            LibrePodsTheme {
                var visible by remember { mutableStateOf(false) }
                LaunchedEffect(Unit) { visible = true }
                Box(
                    Modifier
                        .fillMaxSize()
                        .clickable(remember { MutableInteractionSource() }, null) { finish() },
                    contentAlignment = Alignment.Center
                ) {
                    AnimatedVisibility(
                        visible = visible,
                        enter = fadeIn(tween(200)) + scaleIn(tween(250), initialScale = 0.92f),
                    ) {
                        Column(
                            Modifier
                                .padding(horizontal = 24.dp)
                                .widthIn(max = 420.dp)
                                .fillMaxWidth()
                                .clip(RoundedCornerShape(28.dp))
                                .background(MaterialTheme.colorScheme.surfaceContainerHigh)
                                // swallow taps on the dialog itself
                                .clickable(remember { MutableInteractionSource() }, null) {}
                                .padding(top = 24.dp, bottom = 18.dp, start = 24.dp, end = 24.dp),
                            horizontalAlignment = Alignment.CenterHorizontally
                        ) {
                            Icon(
                                painterResource(R.drawable.ic_w_earbuds), null,
                                tint = MaterialTheme.colorScheme.onSurface,
                                modifier = Modifier.size(28.dp)
                            )
                            Text(
                                stringResource(R.string.ui_qs_title),
                                style = MaterialTheme.typography.headlineSmall,
                                color = MaterialTheme.colorScheme.onSurface,
                                modifier = Modifier.padding(top = 14.dp)
                            )
                            Text(
                                stringResource(R.string.ui_qs_subtitle),
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(top = 4.dp, bottom = 20.dp)
                            )
                            QsDevices()
                            Row(
                                Modifier.fillMaxWidth().padding(top = 20.dp),
                                horizontalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                OutlinedButton(onClick = {
                                    startActivity(Intent(this@QuickSettingsDialogActivity, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                                    finish()
                                }) { Text(stringResource(R.string.ui_qs_settings)) }
                                Spacer(Modifier.weight(1f))
                                Button(onClick = { finish() }) { Text(stringResource(R.string.ui_qs_done)) }
                            }
                        }
                    }
                }
            }
        }
    }
}

private class QsDevice(
    val name: String,
    val isMax: Boolean,
    val battery: List<Battery>,
    val mode: Int,
    val showOff: Boolean,
    val showAdaptive: Boolean,
    val aacp: AACPManager,
    val isMain: Boolean,
    val mac: String? = null,
)

private fun AACPManager.modeAndOff(): Pair<Int?, Boolean> {
    val list = controlCommandStatusList.toList()
    val mode = list.find { it.identifier == ControlCommandIdentifiers.LISTENING_MODE }?.value?.firstOrNull()?.toInt()
    val off = list.find { it.identifier == ControlCommandIdentifiers.ALLOW_OFF_OPTION }?.value?.firstOrNull() == 0x01.toByte()
    return mode to off
}

private fun currentDevices(context: android.content.Context): List<QsDevice> {
    val service = ServiceManager.getService() ?: return emptyList()
    val devices = mutableListOf<QsDevice>()
    if (BluetoothConnectionManager.aacpSocket?.isConnected == true) {
        val model = service.airpodsInstance?.model
        val (mode, off) = service.aacpManager.modeAndOff()
        val prefs = context.getSharedPreferences("settings", android.content.Context.MODE_PRIVATE)
        devices += QsDevice(
            name = prefs.getString("name", null) ?: model?.displayName ?: "AirPods",
            isMax = model is AirPodsMax,
            battery = service.getBattery(),
            mode = WidgetRenderer.displayedMode(null, mode ?: service.getANC()),
            showOff = off || prefs.getBoolean("off_listening_mode", true),
            showAdaptive = model == null || Capability.ADAPTIVE_AUDIO in model.capabilities,
            aacp = service.aacpManager,
            isMain = true,
        )
    }
    CompanionSessions.all().forEach { c ->
        val model = c.instance?.model
        val (mode, off) = c.aacpManager.modeAndOff()
        devices += QsDevice(
            name = c.name ?: "AirPods",
            isMax = model is AirPodsMax,
            battery = c.batteryNotification.getBattery(),
            mode = WidgetRenderer.displayedMode(c.device.address, mode ?: 2),
            showOff = off,
            showAdaptive = model == null || Capability.ADAPTIVE_AUDIO in model.capabilities,
            aacp = c.aacpManager,
            isMain = false,
            mac = c.device.address,
        )
    }
    return devices
}

@Composable
private fun QsDevices() {
    val context = LocalContext.current
    var tick by remember { mutableLongStateOf(0L) }
    LaunchedEffect(Unit) {
        while (true) {
            delay(700)
            tick++
        }
    }
    val devices = remember(tick) { currentDevices(context) }
    if (devices.isEmpty()) {
        Text(
            stringResource(R.string.widget_not_connected),
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(8.dp)
        )
        return
    }
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        devices.forEach { d ->
            val level = d.battery
                .filter { it.status != BatteryStatus.DISCONNECTED && it.component != BatteryComponent.CASE && it.level > 0 }
                .minOfOrNull { it.level }
            Column(
                Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(24.dp))
                    .background(if (d.isMain) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceContainerHighest)
                    .padding(start = 16.dp, end = 16.dp, top = 14.dp, bottom = 12.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                val fg = if (d.isMain) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurface
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    Icon(
                        painterResource(if (d.isMax) R.drawable.ic_w_headphones else R.drawable.ic_w_earbuds), null,
                        tint = fg, modifier = Modifier.size(24.dp)
                    )
                    Column(Modifier.weight(1f)) {
                        Text(d.name, style = MaterialTheme.typography.titleMedium, color = fg, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        Text(
                            listOfNotNull(
                                stringResource(if (d.isMain) R.string.ui_qs_active else R.string.ui_qs_connected),
                                level?.let { stringResource(R.string.ui_qs_battery, it) },
                                modeLabel(d.mode),
                            ).joinToString(", "),
                            style = MaterialTheme.typography.bodyMedium,
                            color = fg.copy(alpha = 0.8f),
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis
                        )
                    }
                }
                ListeningModeGroup(
                    options = listeningModeOptions(d.showOff, d.showAdaptive),
                    selected = d.mode,
                    height = 48.dp,
                    showLabels = false,
                    onSelect = { mode ->
                        if (d.isMain) ServiceManager.getService()?.ancNotification?.setStatus(byteArrayOf(mode.toByte()))
                        WidgetRenderer.setModeOptimistically(context, d.mac, mode)
                        Thread {
                            d.aacp.sendControlCommand(ControlCommandIdentifiers.LISTENING_MODE.value, mode.toByte())
                        }.start()
                        tick++
                    }
                )
            }
        }
    }
}

@Composable
private fun modeLabel(mode: Int) = stringResource(
    when (mode) {
        1 -> R.string.widget_noise_control_off
        3 -> R.string.transparency
        4 -> R.string.adaptive
        else -> R.string.noise_cancellation
    }
)
