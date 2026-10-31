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

@file:OptIn(ExperimentalEncodingApi::class)

package me.kavishdevar.librepods.services

import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothSocket
import android.content.Context
import android.content.Intent
import android.os.ParcelUuid
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout
import me.kavishdevar.librepods.bluetooth.AACPManager
import me.kavishdevar.librepods.bluetooth.createBluetoothSocket
import me.kavishdevar.librepods.data.AirPodsInstance
import me.kavishdevar.librepods.data.AirPodsModels
import me.kavishdevar.librepods.data.AirPodsNotifications
import me.kavishdevar.librepods.data.Capability
import me.kavishdevar.librepods.data.CustomEq
import kotlin.io.encoding.ExperimentalEncodingApi
import kotlin.time.Duration.Companion.milliseconds

/**
 * A second (third, …) pair of AirPods connected at the same time as the main one.
 *
 * The main device keeps the full feature set in [AirPodsService] (takeover, ear detection,
 * gestures, head tracking). A companion only gets its own AACP link for what makes sense on
 * a device that isn't the active audio route: battery, listening mode and control commands.
 */
class CompanionSession(val device: BluetoothDevice) {
    val mac: String get() = device.address
    val aacpManager = AACPManager()
    val batteryNotification = AirPodsNotifications.BatteryNotification()
    @Volatile var socket: BluetoothSocket? = null
    @Volatile var instance: AirPodsInstance? = null
    @Volatile var name: String? = null

    val isConnected: Boolean get() = socket?.isConnected == true

    init {
        aacpManager.socketProvider = { socket }
    }

    fun close() {
        runCatching { socket?.close() }
        socket = null
        aacpManager.disconnected()
    }
}

/** Find My appends " - Find My" to the Bluetooth name; the pair's own name is what's before it. */
fun cleanDeviceName(name: String): String = name.replace(Regex("\\s*-\\s*Find M.*$"), "")

/** Keeps track of companion devices and notifies listeners whenever any of them changes. */
object CompanionSessions {
    private const val TAG = "CompanionSessions"
    const val ACTION_UPDATED = "me.kavishdevar.librepods.COMPANIONS_UPDATED"

    private val sessions = java.util.concurrent.ConcurrentHashMap<String, CompanionSession>()
    private val _changes = MutableStateFlow(0L)

    /** Bumps whenever a companion connects, disconnects or reports new data. */
    val changes: StateFlow<Long> get() = _changes

    fun all(): List<CompanionSession> = sessions.values.filter { it.isConnected }.sortedBy { it.mac }

    fun get(mac: String?): CompanionSession? = mac?.let { sessions[it] }?.takeIf { it.isConnected }

    fun contains(mac: String) = sessions.containsKey(mac)

    /** Detaches a companion so the service can take it over as the main device. */
    fun remove(mac: String): CompanionSession? = sessions.remove(mac)?.also { it.close() }

    fun changed(context: Context) {
        _changes.value = _changes.value + 1
        context.sendBroadcast(Intent(ACTION_UPDATED).setPackage(context.packageName))
        me.kavishdevar.librepods.presentation.widgets.WidgetRenderer.updateAll(context)
        ServiceManager.getService()?.postCompanionCards()
    }

    @SuppressLint("MissingPermission")
    fun connect(context: Context, adapter: BluetoothAdapter, device: BluetoothDevice, retry: Boolean = true) {
        val session = CompanionSession(device)
        // Already connecting or connected to this one.
        if (sessions.putIfAbsent(device.address, session) != null) return
        session.name = runCatching { device.name }.getOrNull()?.let(::cleanDeviceName)
        CoroutineScope(Dispatchers.IO).launch {
            val socket = try {
                createBluetoothSocket(adapter, device, ParcelUuid.fromString("74ec2172-0bad-4d01-8f77-997b2be0722a"), 4097)
            } catch (e: Exception) {
                Log.w(TAG, "Couldn't create socket for ${device.address}: ${e.message}")
                sessions.remove(device.address, session)
                return@launch
            }
            try {
                withTimeout(5000.milliseconds) { socket.connect() }
            } catch (e: Throwable) {
                Log.w(TAG, "Couldn't connect to ${device.address}: ${e.message}")
                runCatching { socket.close() }
                sessions.remove(device.address, session)
                if (retry) {
                    delay(2000)
                    connect(context, adapter, device, retry = false)
                }
                return@launch
            }
            session.socket = socket
            session.aacpManager.setPacketCallback(callbacks(context, session))
            Log.d(TAG, "Companion connected: ${device.address}")
            changed(context)

            val aacp = session.aacpManager
            aacp.sendPacket(aacp.createHandshakePacket())
            delay(200)
            aacp.sendSetFeatureFlagsPacket()
            delay(200)
            aacp.sendNotificationRequest()

            val buffer = ByteArray(1024)
            try {
                while (socket.isConnected) {
                    val read = socket.inputStream.read(buffer)
                    if (read <= 0) break
                    aacp.receivePacket(buffer.copyOfRange(0, read))
                }
            } catch (e: Exception) {
                Log.d(TAG, "Companion ${device.address} read ended: ${e.message}")
            }
            if (sessions[device.address] === session) {
                sessions.remove(device.address)
                session.close()
                Log.d(TAG, "Companion disconnected: ${device.address}")
                changed(context)
            }
        }
    }

    fun disconnectAll(context: Context) {
        sessions.values.forEach { it.close() }
        sessions.clear()
        changed(context)
    }

    private fun callbacks(context: Context, session: CompanionSession) = object : AACPManager.PacketCallback {
        override fun onBatteryInfoReceived(batteryInfo: ByteArray) {
            session.batteryNotification.setBattery(batteryInfo)
            changed(context)
        }

        override fun onControlCommandReceived(controlCommand: ByteArray) = changed(context)

        override fun onDeviceInformationReceived(deviceInformation: AACPManager.Companion.AirPodsInformation) {
            session.name = deviceInformation.name
            AirPodsModels.getModelByModelNumber(deviceInformation.modelNumber)?.let { model ->
                session.instance = AirPodsInstance(
                    name = deviceInformation.name,
                    model = model,
                    actualModelNumber = deviceInformation.modelNumber,
                    serialNumber = deviceInformation.serialNumber,
                    leftSerialNumber = deviceInformation.leftSerialNumber,
                    rightSerialNumber = deviceInformation.rightSerialNumber,
                    version1 = deviceInformation.version1,
                    version2 = deviceInformation.version2,
                    version3 = deviceInformation.version3,
                )
            }
            changed(context)
        }

        override fun onEarDetectionReceived(earDetection: ByteArray) {}
        override fun onConversationAwarenessReceived(conversationAwareness: ByteArray) {}
        override fun onHeadTrackingReceived(headTracking: ByteArray) {}
        override fun onUnknownPacketReceived(packet: ByteArray) {}
        override fun onProximityKeysReceived(proximityKeys: ByteArray) {}
        override fun onStemPressReceived(stemPress: ByteArray) {}
        override fun onAudioSourceReceived(audioSource: ByteArray) {}
        override fun onOwnershipChangeReceived(owns: Boolean) {}
        override fun onConnectedDevicesReceived(connectedDevices: List<AACPManager.Companion.ConnectedDevice>) {}
        override fun onOwnershipToFalseRequest(sender: String, reasonReverseTapped: Boolean) {}
        override fun onShowNearbyUI(sender: String) {}
        override fun onHeadphoneAccommodationReceived(eqData: FloatArray) {}
        override fun onCustomEqReceived(customEq: CustomEq) {}
        override fun onCapabilitiesReceived(capabilities: List<Capability>) {}
    }
}
