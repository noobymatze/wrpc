# wrpc

wrpc is a language for describing APIs. It can be used to generate clients,
server interfaces (including validation), documentation, mock services and
tests.


## Why should you care?

Perhaps you have also been part of endless meetings about specifying an HTTP
API (or REST API if you will), with different people having different opinions
about how URLs should look like, what constitutes a RESTful API, which status
codes are appropriate for which failure modes. Disagreements about a particular
status code for a particular failure mode, people getting angry at each other,
because while they do agree, that the right status code is "obvious", they just
don't agree on the status code. Product people trying to throw in additional
constraints until you try to defuse the situation by half jokingly suggesting
to use the 418 status code, at which point all involved turn to you as their
common enemy.

I might have been slightly exaggerating that last part. However, the reality
is, that in those kinds of meetings we are so often discussing the unimportant
minutiae. We are not discussing what the service should do or what the actual
failure modes could be, but how to translate these failure modes into a
technical standard. Yes, that's part of our job, but we can also go just too
far. I can't imagine 

I envision a meeting like that, where backend and frontend developers and
product people describe the contract in unison and then frontend developers
generate heaps of code to 

## Example

```wrpc
enum PaymentMethod {
    Credit { 
        #(check (not blank) (< .length 120))
        name: String 
    },
    PayPal,
}
```


## Language

## Protocol

## Comparison

### OpenAPI

OpenAPI is the de-facto standard for generating 

### gRPC

### Smithy

### Typespec



[smithy]: https://smithy.io/2.0/index.html
[grpc]: https://grpc.io/
[typespec]: https://typespec.io/
