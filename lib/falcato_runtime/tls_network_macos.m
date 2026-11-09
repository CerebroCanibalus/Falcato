//! # TLS en macOS — shim Objective-C sobre Network.framework
//!
//! Network.framework (la API TLS moderna de Apple) es **asíncrona y basada en
//! bloques**: el handshake y el I/O se reportan por *completion handlers*.
//! La ABI de Falcato es **síncrona** (por ejemplo `falcato_tls_leer` debe
//! bloquear hasta tener datos), así que este shim traduce en un solo sentido:
//!
//!     Rust  ──►  falcato_nw_tls_*  (C, bloqueante)  ──►  Network.framework
//!
//! No implementa criptografía: la conexión, el handshake TLS, el SNI y la
//! validación del certificado los hace el sistema (evaluación de confianza por
//! defecto contra el Keychain). Solo puentea sincronía con `dispatch_semaphore`.
//!
//! ## Nota de seguridad
//! - No se desactiva la verificación de certificados en ningún punto.
//! - Host y puerto se validan antes de usarse; el host se copia y se cierra con
//!   NUL (no se asume terminación en el origen).
//! - Todos los `receive`/`send` tienen timeout propio: una conexión colgada no
//!   bloquea al runtime para siempre.

#import <Foundation/Foundation.h>
#import <Network/Network.h>
#import <Security/SecProtocolOptions.h>
#import <dispatch/dispatch.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const int64_t TIMEOUT_CONECTAR_MS = 15000;
static const int64_t TIMEOUT_ESCRIBIR_MS = 15000;
static const int64_t TIMEOUT_LEER_MS = 30000;

/// Estado de una conexión TLS vista desde Rust.
@interface FalcatoTls : NSObject
@property (nonatomic, strong) nw_connection_t conexion;
@property (nonatomic, strong) dispatch_queue_t cola;
@property (nonatomic, assign) BOOL cerrado;
@end

@implementation FalcatoTls
@end

static dispatch_time_t falcato_en_ms(int64_t ms) {
    return dispatch_time(DISPATCH_TIME_NOW, ms * NSEC_PER_MSEC);
}

/// Copia hasta `n` bytes de un `dispatch_data` a `buf`. Devuelve los copiados.
static int32_t falcato_copiar_datos(dispatch_data_t datos, void *buf, int32_t n) {
    if (datos == nil || buf == NULL || n <= 0) {
        return -1;
    }
    const void *region = NULL;
    size_t tam = 0;
    dispatch_data_t mapeado = dispatch_data_create_map(datos, &region, &tam);
    if (mapeado == nil || region == NULL) {
        return -1;
    }
    size_t copia = tam < (size_t)n ? tam : (size_t)n;
    if (copia > 0) {
        memcpy(buf, region, copia);
    }
    return (int32_t)copia;
}

/// Conecta y hace el handshake TLS. Devuelve handle opaco o NULL.
/// `host` NO necesita estar terminado en NUL (se copia con `host_len`).
void *falcato_nw_tls_conectar(const char *host, size_t host_len, int32_t puerto) {
    if (host == NULL || host_len == 0 || host_len > 4096 || puerto <= 0 || puerto > 65535) {
        return NULL;
    }

    char *host_z = (char *)malloc(host_len + 1);
    if (host_z == NULL) {
        return NULL;
    }
    memcpy(host_z, host, host_len);
    host_z[host_len] = '\0';

    char puerto_z[8];
    snprintf(puerto_z, sizeof(puerto_z), "%d", (int)puerto);

    nw_endpoint_t extremo = nw_endpoint_create_host(host_z, puerto_z);
    if (extremo == nil) {
        free(host_z);
        return NULL;
    }

    // `nw_parameters_create_secure_tcp` recibe BLOQUES de configuración (se
    // ejecutan de forma síncrona acá). Dentro se configura el SNI; la
    // verificación de certificado queda en su valor por defecto: el sistema
    // evalúa la cadena contra el Keychain. No se desactiva nada.
    nw_parameters_t parametros = nw_parameters_create_secure_tcp(
        ^(nw_protocol_options_t opciones) {
            sec_protocol_options_t seg = nw_tls_copy_sec_protocol_options(opciones);
            if (seg != nil) {
                sec_protocol_options_set_tls_server_name(seg, host_z);
            }
        },
        NW_PARAMETERS_DEFAULT_CONFIGURATION);
    free(host_z);
    if (parametros == nil) {
        return NULL;
    }
    nw_connection_t conexion = nw_connection_create(extremo, parametros);
    if (conexion == nil) {
        return NULL;
    }

    FalcatoTls *estado = [[FalcatoTls alloc] init];
    estado.conexion = conexion;
    estado.cola = dispatch_queue_create("falcato.tls", DISPATCH_QUEUE_SERIAL);

    dispatch_semaphore_t espera = dispatch_semaphore_create(0);
    __block int resultado_estado = -1;
    nw_connection_set_state_changed_handler(conexion, ^(nw_connection_state_t estado_nw, nw_error_t error) {
        (void)error;
        if (estado_nw == nw_connection_state_ready) {
            resultado_estado = 1;
            dispatch_semaphore_signal(espera);
        } else if (estado_nw == nw_connection_state_failed || estado_nw == nw_connection_state_cancelled) {
            resultado_estado = -1;
            dispatch_semaphore_signal(espera);
        }
    });
    nw_connection_set_queue(conexion, estado.cola);
    nw_connection_start(conexion);

    if (dispatch_semaphore_wait(espera, falcato_en_ms(TIMEOUT_CONECTAR_MS)) != 0 || resultado_estado != 1) {
        nw_connection_cancel(conexion);
        return NULL;
    }

    return (__bridge_retained void *)estado;
}

/// Escribe `n` bytes. Devuelve los escritos o -1.
int32_t falcato_nw_tls_escribir(void *handle, const void *datos, int32_t n) {
    if (handle == NULL || datos == NULL || n <= 0) {
        return -1;
    }
    FalcatoTls *estado = (__bridge FalcatoTls *)handle;
    if (estado.cerrado || estado.conexion == nil) {
        return -1;
    }

    // DESTRUCTOR_DEFAULT: Network copia los bytes, así el caller puede liberar.
    dispatch_data_t bloque = dispatch_data_create(datos, (size_t)n, estado.cola, DISPATCH_DATA_DESTRUCTOR_DEFAULT);
    if (bloque == nil) {
        return -1;
    }

    dispatch_semaphore_t espera = dispatch_semaphore_create(0);
    __block int32_t escritos = -1;
    nw_connection_send(estado.conexion, bloque, NW_CONNECTION_DEFAULT_MESSAGE_CONTEXT, true, ^(nw_error_t error) {
        escritos = (error == NULL) ? n : -1;
        dispatch_semaphore_signal(espera);
    });

    if (dispatch_semaphore_wait(espera, falcato_en_ms(TIMEOUT_ESCRIBIR_MS)) != 0) {
        estado.cerrado = YES;
        return -1;
    }
    return escritos;
}

/// Lee hasta `n` bytes (bloqueante). Devuelve bytes leídos, 0 en EOF, -1 en error.
int32_t falcato_nw_tls_leer(void *handle, void *buf, int32_t n) {
    if (handle == NULL || buf == NULL || n <= 0) {
        return -1;
    }
    FalcatoTls *estado = (__bridge FalcatoTls *)handle;
    if (estado.cerrado || estado.conexion == nil) {
        return 0;
    }

    dispatch_semaphore_t espera = dispatch_semaphore_create(0);
    __block int32_t leidos = -1;
    nw_connection_receive(estado.conexion, 1, (size_t)n,
                          ^(dispatch_data_t contenido, nw_content_context_t contexto,
                            bool completo, nw_error_t error) {
        (void)contexto;
        if (error != NULL) {
            leidos = -1;
        } else if (contenido != nil) {
            leidos = falcato_copiar_datos(contenido, buf, n);
        } else if (completo) {
            leidos = 0;
        }
        dispatch_semaphore_signal(espera);
    });

    if (dispatch_semaphore_wait(espera, falcato_en_ms(TIMEOUT_LEER_MS)) != 0) {
        estado.cerrado = YES;
        return -1;
    }
    if (leidos == 0) {
        estado.cerrado = YES;
    }
    return leidos;
}

/// ¿Hay datos para leer? Network.framework no expone "bytes disponibles" sin
/// consumirlos, así que se responde de forma conservadora: conexión viva = 1.
int32_t falcato_nw_tls_datos_disponibles(void *handle) {
    if (handle == NULL) {
        return 0;
    }
    FalcatoTls *estado = (__bridge FalcatoTls *)handle;
    if (estado.cerrado || estado.conexion == nil) {
        return 0;
    }
    return 1;
}

/// Cierra la conexión y libera el estado (consume la referencia de Rust).
void falcato_nw_tls_cerrar(void *handle) {
    if (handle == NULL) {
        return;
    }
    FalcatoTls *estado = (__bridge_transfer FalcatoTls *)handle;
    estado.cerrado = YES;
    if (estado.conexion != nil) {
        nw_connection_cancel(estado.conexion);
        estado.conexion = nil;
    }
}
