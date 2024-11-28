:sunny: :cloud: :construction_worker_man: :construction::construction::construction::construction::construction::construction:

# wRPC

At some undefined point in the future, wRPC might be an interface
description language (IDL) for describing APIs, much like
[OpenAPI][openapi] or [gRPC][grpc]. At that undefined point in the
future it might be used to generate clients, server interfaces
(including validation), documentation, run mock servers and even run
tests.

## Example

Here is 

```wrpc
// This is a `Person`.
data Person {
    name: String,
    age: Int?,
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
translate the failure modes into a technical standard. Yes, that's
part of our job, but I don't think it should be and especially not
during that kind of meeting.

I envision a meeting like that, where backend, frontend and product
people describe the contract in unison and then frontend developers
generate heaps of code to interface with the backend server and
backend developers generate heaps of code to interface with the
frontend and they can just implement it.

## Language

The following section describes the foundations of the wRPC language, which
allows you to describe APIs between services or services and frontends.

### Record

A record is a group of named values with different types. A record always has a
name itself. It can be thought of as a tuple, that has a name for each
component of the tuple.

```rust
data Person {
    name: String,
    age: Int32,
}
```

On the wire, such a structure will be converted to the following JSON representation:

```json
{
    "name": "Test",
    "age": 32,
}
```


### Enum

An enum is set of differing values, potentially with associated data. 

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
following representation:

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

Enums can also be defined without any associated data.

```rust
enum Role {
    Admin,
    User,
}
```

In this case the wire representation will look like the following:

```
"Admin"
```

If at least one variant of an `enum` contains associated data, a
discriminator is used for every variant.


### Service

A `Service` defines a set of methods, the server has to implement to
serve to a client.

```wrpc
service PersonService {
    def get(id: Int64): Person?
}
```

### Builtins

The following structures are built-in for you.

#### Result<E, A>

A `Result` represents the result of a computation. If the computation was
successful, it will contain a value of type `A`. If the computation was
unsuccessful it will contain a value of type `E`.

A `Result` can be defined as follows in wrpc:

```rust
enum Result<E, A> {
    Ok { value: A },
    Err { error: E },
}
```

For languages where a language level `Result` already exists (e.g. Rust), there
will be an extra one generated.

#### List<A>

A `List<A>` represents a sequence of values of type `A`. A list will
be represented on the wire as 

```rust
[1, 2, 3]
```


## Protocol

TODO

## Comparison

TODO

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

