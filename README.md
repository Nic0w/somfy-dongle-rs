# Client for Enki "Somfy RTS" extension dongle

As part of its "smart home" offering, Enki, a sub-brand of french group Leroy Merlin, [is selling a USB dongle](https://www.leroymerlin.fr/produits/chauffage-plomberie/chauffage-electrique/radiateur-electrique/pack-connecte-pour-radiateur-electrique/cle-extension-somfy-rts-pour-box-enki-82292229.html) to add RTS compatibility to their eponymous smart home box.

Normally, the dongle only works with the proprietary Enki box. This code base aims to bring compatibility with the dongle to any machine with a modern OS and USB port.

## How does it work

### RTS protocol
RTS is a proprietary radio protocol from french company Somfy, which is primarily used to command motorized rolling shutters, sun blinds, ...
The protocol itself [has been reverse-enginerred](https://pushstack.wordpress.com/somfy-rts-protocol/) for a long time now and it has some peculiarities that make it complicated (imo) to work with : working on 433.42MHz (instead of 443.92Mhz for most/all hardware using the 433Mhz band), it requires specific hardware.
There are however a large pool of ressources on the web regarding open source implementations for Arduino, Raspberry Pi, ...

### Enki's dongle
With the dongle, the hardware is already made for us, and nicely packaged in a compact form factor, as a USB stick !\
Basically, a stm32 micro-controller exposes a serial port device over USB. There is a custom protocol to send commands and thus we don't have to deal with RTS at all.

## This code base
This code base is written in Rust and comprises of multiple tools:
 - a library implementing I/O over the serial port. This is ths core, with the custom protocol open source implementation.
 - a CLI application, `somfy-rts-cli`, whose aim is to provide a way to send discrete commands to RTS-able objects
 - a service, `somfy-rts-mqtt`, which is used to provide a one-way bridge to MQTT (and compatibilty with Home Assistant, through its MQTT auto-discovery feature !)

## How-to

### Register a RTS object

The first step is to register an object with the dongle.
This mostly works the same way than [registering a new remote](https://www.somfysystems.com/en-us/support/faq?question=how-to-add-or-remove-additional-rts-controls-for-your-rolling-shutter):
 1. (For a first use, using `somfy-rts-cli`, it is advised to zero out the dongle's storage :)
 ```sh
 $ target/release/somfy-rts-cli reset-addres 1..100
 ```
 2. Press the 'Programming' button on your existing remote. There will be some movement, this is the object acknowledging it is in programming mode

 3. Pair the dongle with the object :
 ```sh
 $ target/release/somfy-rts-cli prog 1
 ```

There will be movement again, confirming the binding did occur. To pair other devices, the procedure needs to be repeated and the id incremented. For example for a second object:
 ```sh
 $ target/release/somfy-rts-cli prog 2
 ```

The dongle can interact with up to 100 objects.

### MQTT bridging
`somfy-rts-mqtt` is used to bridge the dongle over MQTT.
For this to work you only need an existing MQTT broker, which is already the case if, for example, you already have Zigbee2MQTT installed for other Smart Home stuff.

Usage is quite simple: 
```sh
 $ /usr/bin/somfy-rts-mqtt -s /dev/ttyACM0 plop:example.com:1883
```

`somfy-rts-mqtt` takes one mandatory argument, which is the MQTT option string used to connect to the MQTT broker. It has the following format:
```<CLIENT_ID>:<BROKER_DOMAIN_NAME>:<PORT>```

Once launched, `somfy-rts-mqtt` will attempt to connect to the dongle, then it will enumerate the registered RTS objects.

For each object, an MQTT endpoint is created: ```somfy-rts/cover/<id>/set``` .\
RTS being a one-way protocol, this endpoint will serve to send commands but no acknowledgment will ever be sent.

The supported commands are:
 - UP
 - DOWN
 - STOP

A ```somfy-rts/dongle/state``` endpoint is also created to convey that the bridge is ```online```.

An example `systemd` .service [is provided](./somfy-rts-mqtt/somfy-rts-mqtt.service) for ease of use as a service on Linux platforms. 

## Home Assistant compatibility

Upon launch, `somfy-rts-mqtt` will also set up the necessary MQTT nodes & endpoints to leverage Home Assistant MQTT auto-discovery.

As such, each object enumerated during launch should register as its own [Cover entity](https://www.home-assistant.io/integrations/cover/) in Home Assistant.
RTS being only a one-way protocol, the set of features is somewhat limited but the main ones are working: open, close, stop.

# DISCLAIMER
The developer(s) would like to clarify that they have no affiliation with Somfy, Enki, or Leroy Merlin. The software provided is independently developed and does not involve any collaboration or endorsement from these companies. It is important to note that there is no warranty or guarantee provided with the software. While efforts have been made to ensure its functionality and reliability, the developer(s) cannot guarantee its performance or suitability for any specific purpose. Users are advised to utilize the software at their own discretion and risk.


