# Sistema de Semántica y Tipos de Falcato (borrador)

## 1. Sistema de Tipos

### Primitivos

| Tipo | Descripción | Tamaño |
|------|-------------|--------|
| `Byte` | Entero sin signo de 8 bits | 8 bits |
| `Entero{8,16,32,64}` | Entero con signo | 8-64 bits |
| `Natural{8,16,32,64}` | Entero sin signo | 8-64 bits |
| `Flotante{32,64}` | Punto flotante IEEE 754 | 32/64 bits |
| `Booleano` | `cierto` / `falso` | 8 bits (1 usado) |
| `Caracter` | Unicode scalar | 32 bits |
| `Palabra` | Cadena UTF-8 (inmutable) | Variable |
| `Vacio` | Tipo unidad (void) | 0 bits |
| `Nulo` | Null pointer / bottom type | Depende del target |

### Compuestos

```falcat
// Estructura
estructural Punto {
    el x: Flotante64,
    el y: Flotante64,
}

// Enumeración (tagged union)
enumeración Resultado<T, E> {
    Bien(T),
    Arf(E),
}

// Tupla
el par: (Entero32, Palabra) = (42, "sentido");
```

### Genéricos

```falcat
// Convención = trait/interface
convención Comparable<T> {
    función comparar(&la self, &la otro: T): Entero32;
}

// Implementación con where
estructural Caja<T>
donde T: Comparable<T> {
    el valor: T,
}
```

## 2. Modelo de Memoria

### Heap y Stack

Falcato distingue tres regiones de memoria:

```falcat
// Stack — tamaño conocido en compile-time
estructural en_pila {
    el datos: [Byte; 256],
}

// Heap — Box/único propietario
el caja: Caja<Entero32> = Caja::nuevo(42);

// Heap — referencia contada
los nodos: Contado<Lista<Nodo>> = Contado::nuevo(Lista::nuevo());
```

### Tiempos de vida (lifetimes)

Los artículos también pueden expresar tiempos de vida relacionales:

```falcat
función mayor<'la>(a: &'la Entero32, b: &'la Entero32) -> &'la Entero32 {
    si a > b { a } sino { b }
}
```

### Mutabilidad

```falcat
ser x: Entero32 = 10;      // compile-time constant
estar y: Entero32 = 20;     // runtime mutable
el z: Entero32 = 30;        // owned, mutable por defecto
la w: &Entero32 = &z;       // borrowed, inmutable
```

## 3. Concurrencia

### Hilos y canales

```falcat
usar std::hilo;
usar std::canal;

función principal() {
    el (emisor, receptor) = canal::nuevo<Entero32>();
    
    hilo::nuevo(mueve emisor, || {
        emisor.enviar(42);
    });
    
    el mensaje = receptor.recibir();  // 42
}
```

### Async (tiempo futuro)

```falcat
función leer_archivo(ruta: Palabra): Futuro<[Byte]> {
    // Cuerpo async
    hacer_io(ruta).esperar
}
```

### Sync (Send + Sync)

```falcat
inseguro convención EnviarEntreHilos {}
inseguro convención CompartirEntreHilos {}

// Auto-derivado para tipos puramente owned
// No implementado para tipos con la (borrowed) interna
```

## 4. Contratos en tiempo de compilación

```falcat
con_tal_que(T: Entero32)  // precondition: T debe ser entero

ya_que(tamaño > 0)        // assertion compile-time

a_menos_que(ptr sea nulo)  // guard: si ptr es nulo, no ejecuta
```

## 5. Manejo de errores

```falcat
función dividir(a: Entero32, b: Entero32): Result<Entero32> {
    si b sea 0 {
        fallar(Error("división por cero"));
    }
    retornar a / b;
}

// Uso con subjuntivo
// dividir(10, 2) → bien(5)
// dividir(10, 0) → arf(Error)
```

## 6. Interoperabilidad con C (FFI Day-0)

### Principio: C ABI por defecto

Falcato usa **ABI de C como comportamiento base** en toda la salida del
compilador. No es un opt-in como `extern "C"` en Rust. Esto significa:

- Layout de structs sigue las reglas de C (mismo padding y alineación)
- Calling convention por defecto es `C`
- Name mangling desactivado (símbolos se exportan con su nombre literal)
- Cualquier `.o` generado por Falcato se linkea directamente con `gcc`/`clang`/`link.exe`

### Declaración de funciones externas

```falcat
// Declarar una función de la libc
inseguro función puts(mensaje: &Caracter): Entero32;

// Con tipos C explícitos
inseguro función malloc(tamaño: Natural64): *Vacio;
inseguro función free(ptr: *Vacio): Vacio;
inseguro función printf(formato: *const Caracter, ...): Entero32;
```

### Mapeo de tipos C → Falcato

| Tipo C | Tipo Falcato | Notas |
|--------|-------------|-------|
| `char` | `Byte` | 8 bits, sin signo |
| `signed char` | `Entero8` | |
| `unsigned char` | `Natural8` | |
| `short` | `Entero16` | |
| `unsigned short` | `Natural16` | |
| `int` | `Entero32` | |
| `unsigned int` | `Natural32` | |
| `long` | `Entero32` o `Entero64` | Depende de plataforma |
| `unsigned long` | `Natural32` o `Natural64` | Depende de plataforma |
| `long long` | `Entero64` | |
| `float` | `Flotante32` | |
| `double` | `Flotante64` | |
| `void` | `Vacio` | |
| `void*` | `*Vacio` | Puntero opaco |
| `char*` / `const char*` | `*Caracter` / `*const Caracter` | Puntero a caracteres |
| `size_t` | `Natural64` (x64) / `Natural32` (x86) | Depende del target |
| `int32_t` | `Entero32` | stdint.h directo |

### Estructuras compatibles con C

Por defecto, `estructural` usa el mismo layout que `struct` en C:

```falcat
// Mismo layout que:
// struct Punto { double x; double y; };
estructural Punto {
    el x: Flotante64,
    el y: Flotante64,
}

// Para structs C con padding explícito o packed:
#[repr("C")]  // explícito (redundante, es el default)
estructural Alineado {
    el a: Entero8,
    el b: Entero32,  // padding de 3 bytes aquí
}

#[repr("packed")]
estructural Empaquetado {
    el a: Entero8,
    el b: Entero32,  // sin padding
}
```

### Punteros y memoria compartida

```falcat
// Puntero crudo (sin garantías de ownership)
inseguro función memcpy(dest: *Vacio, src: *const Vacio, n: Natural64): *Vacio;

// Puntero nulo
inseguro función nullable: *Entero32 {
    retornar nulo;
}

// El acceso a punteros requiere inseguro
inseguro {
    el ptr = malloc(64);
    si ptr sea nulo { fallar("sin memoria"); }
    // usar ptr...
    free(ptr);
}
```

### Linkage y compilación

```falcat
// Linkear con una biblioteca C
#[link("m")]   // libm (math)
inseguro función sqrt(x: Flotante64): Flotante64;

#[link("pthread")]
inseguro función pthread_create(...): Entero32;

// Linkear con biblioteca local
#[link("ruta:./libs/mi_biblioteca.a")]
inseguro función mi_funcion(): Entero32;
```

### Módulos FFI completos

```falcat
inseguro módulo libc {
    // Entrada/salida
    función puts(s: *const Caracter): Entero32;
    función printf(fmt: *const Caracter, ...): Entero32;
    
    // Memoria
    función malloc(tamaño: Natural64): *Vacio;
    función calloc(n: Natural64, tamaño: Natural64): *Vacio;
    función free(ptr: *Vacio): Vacio;
    
    // Archivos
    función fopen(ruta: *const Caracter, modo: *const Caracter): *Archivo;
    función fclose(archivo: *Archivo): Entero32;
    
    // Strings
    función strlen(s: *const Caracter): Natural64;
    función strcmp(s1: *const Caracter, s2: *const Caracter): Entero32;
}

usar libc::*;

función principal(): Entero32 {
    puts("Falcato forja poder");
    retornar 0;
}
```

### Generación de bindings automáticos (Fase 2+)

```falcat
// Futuro: generar bindings desde un header C
#[generar_bindings("ruta:include/sdl.h")]
inseguro módulo sdl;
```

### Lo que NO se permite en FFI seguro

- Llamar a funciones FFI fuera de bloques `inseguro`
- Dereferenciar punteros crudos sin marca de seguridad
- Convertir tipos entre C y Falcato sin `como` explícito

## 7. Metaprogramación

### Tiempo de compilación (comptime)

```falcat
ser función TAMAÑO_BUFFER(entradas: []Tipo): Entero32 {
    retornar entradas.longitud * 64;
}

// Se ejecuta en compile-time
el buffer: [Byte; TAMAÑO_BUFFER([...])];
```
