:sunny: :cloud: :construction_worker_man: :construction::construction::construction::construction::construction::construction:

# wRPC

**WORK IN PROGRESS DO NOT USE, ALSO EXPECT SEVERE BUGS**

At some undefined point in the future, wRPC might be an interface
description language (IDL) for describing APIs and their contract,
much like [OpenAPI][openapi] or [gRPC][grpc]. At that undefined point
in the future, it might be used to generate clients, server interfaces
(including validation), documentation, run mock servers and even run
tests.

## Example

Here is a small example, just defining an API for a `PersonService`.

```wrpc
// This is a `Person`.
data Person {
    name: String,
    age: Int32?,
}

// This is a service working with persons.
service PersonService {
    def get(id: Int64): Person?
}
```


## Why another one?

Perhaps you have also been part of endless meetings about specifying
an HTTP API (or REST API if you will), with different people having
different opinions about how URLs should look like, what constitutes a
RESTful API, which status codes are appropriate for which failure
modes. Disagreements about a particular status code for a particular
failure mode, people getting angry at each other, because while they
do agree, that the right status code is "obvious", they just don't
agree on the specific status code. Product people trying to throw in
additional constraints until you try to defuse the situation by half
jokingly suggesting to use the 418 status code, at which point all
involved turn to you as their common enemy.

I might have been slightly exaggerating that last part. However, the
reality is, that in those kinds of meetings we are so often discussing
the unimportant minutiae. We are not discussing what the service
should do or what the actual failure modes could be, but how to
translate those failure modes into a technical standard. Yes, that's
part of our job, but I don't think it should be to that extend and
especially not during that kind of meeting.

I envision a meeting like that, where backend, frontend and product
people describe the contract in unison and then frontend developers
generate heaps of code to interface with the backend server and
backend developers generate heaps of code to interface with the
frontend and than just get on with their lives.

TODO: Why not OpenAPI or gRPC.

## Language

The following section describes the foundations of the wRPC language, which
allows you to describe APIs between services or services and frontends.

### Primitive types

The following types are primitive types, used as building blocks to
describe an API. 

- `Int32` defines a 32-bit integer type.
- `Int64` defines a 64-bit integer type. 
- `Float32` defines a 32-bit floating point type. 
- `Float64` defines a 64-bit floating point type. 
- `String` defines a UTF-8 encoded String
- `Boolean` defines a boolean type.

#### Correspondence to other languages

The following table shows, how these primitive types will be compiled
down to other languages.

 | wRPC    | Kotlin  | Java    | Rust   | Go     | JS/TS   |
 |:--------|:--------|:--------|:-------|:-------|:--------|
 | Int32   | Int     | int     | i32    | int32  | number  |
 | Int64   | Long    | long    | i64    | int64  | number  |
 | Float32 | Float   | float   | i32    | int32  | number  |
 | Float64 | Double  | double  | i64    | int64  | number  |
 | String  | String  | String  | String | string | string  |
 | Boolean | Boolean | boolean | bool   | bool   | boolean |
 

### Record

A record is a group of named values with different types. A record
always has a name itself. It can be thought of as a tuple, that has a
name for each component of the tuple.

```rust
data Person {
    name: String,
    age: Int32,
}
```

On the wire, such a structure will be converted to the following JSON
representation:

```json
{
    "name": "Test",
    "age": 32
}
```


### Enum

An enum is set of differing values. It is possible to define
associated data for each variant.

#### No associated data

The following enum declaration is an example of an enum without any
associated data.

```rust
enum Role {
    Admin,
    User,
}
```

The wire representation looks like this.

```
"Admin"
```

#### With associated data

The following enum declaration is an example, where each


```rust
enum PaymentMethod {
    PayPal { 
        name: String 
    },
    CreditCard { 
        name: String, 
        cardNo: String, 
        cvc: String, 
        expirationDate: String 
    },
    Sepa { 
        iban: String,
    },
}
```

On the wire, an instance of this enum will be converted to the
following representations:

```json
{ 
    "@type": "PayPal",
    "name": "Test"
}
```

```json
{ 
    "@type": "CreditCard",
    "name": "Test",
    "cardNo": "ABCD",
    "cvc": "180",
    "expirationDate": "26-10",
}
```

```json
{ 
    "@type": "Sepa",
    "iban": "ABCD",
}
```

Since `@type` is not a valid identifier in the wRPC language, its
usage as a discriminator is safe.

If at least one variant of an `enum` contains associated data, a
discriminator is used for every variant.


### Service

A `Service` defines a grouped set of methods

```wrpc
service PersonService {
    def get(id: Int64): Person?
}
```

### Built-ins

The following data structures are built-in, because they require
additional handling of and can be used to compose
bigger structures.

#### Result<E, A>

A `Result` represents the result of a computation. If the computation
was successful, it will contain a value of type `A`. If the
computation was unsuccessful it will contain a value of type `E`.

A `Result` can be defined as follows in wRPC:

```rust
enum Result<E, A> {
    Ok { value: A },
    Err { error: E },
}
```

For languages where a language level `Result` already exists
(e.g. Rust), wRPC will generate a custom `Result` type. This is to
ensure clients and servers can properly communicate with each other.

#### List<A>

A `List<A>` represents a sequence of values of type `A`. A list will
be represented on the wire as a JSON array.

```json
[1, 2, 3]
```

#### Set<A>

A `Set<A>` represents an unordered set of values of type `A`. A set
will be represented on the wire as a JSON array.

```json
[1, 2, 3]
```

#### Map<K, V>

A `Map<K, V>` represents a set of unordered key value pairs, where `K`
is the Key and `V` is the value.

Since not all keys are valid JSON, the wire representation will be a
list of pairs.

```json
[[1, 2],[1, 3]]
```

### Annotations

Almost any element (data, service, enum, variant, method, property,
param, return type) in a wRPC file may contain one or more
annotations.  An annotation is enclosed in `#()` and is an `edn`
value.

Here is an example.

```clojure
// This is a `Person`.
#(check (not (blank .name)))
data Person {
    name: String,
    age: Int32?,
}
```

In this case the `check` annotation defines a constraint for the
`name` property of a `Person`, specifically, that it should not be
blank.

### Comments



## Protocol

If a client sends a request to 
TODO

## Comparison

This section contains 

### OpenAPI

TODO

### gRPC

TODO

### Smithy

TODO

### Typespec

TODO


[smithy]: https://smithy.io/2.0/index.html
[grpc]: https://grpc.io/
[typespec]: https://typespec.io/
[openapi]: https://www.openapis.org/

