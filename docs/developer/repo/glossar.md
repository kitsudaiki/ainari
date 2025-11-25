# Glossar

## Core-Structure

### Neuron

`Neuron` are the smalles entity within the core-structure, which provides the base of the neuronal
network. A `Neuron` is connected one or more `Synapses` to other nodes.

### Synapse

`Synapses` are the connetions between the nodes. They are not static and can be create, destroyed
and modified.

### Hexagon

A `Hexagon` consist of multiple nodes, which are connected by synapses ot nodes in other hexagons.
There are basically 3 types:

1. `Input-Hexagons`

1. `Output-Hexagons`

1. `Core-Hexagons`

### Cluster

`Cluster` are again a collection of multiple `Hexagons`

### Data-Set

A `Data-Set` defines a set of train-data or data, which should be used for requests against the
neuronal network. This can be a table in CSV-format or an MNIST-dataset for now.

## Infrastrure

### Tasks

Tasks an asynchronous operation of the network based on a given `Data-Set`. There are two types:

1. **Train-Task**

    For `Train-Tasks` input-data and desired output must exist in the `Data-Set` in order to update
    the neuronal network based on the data.

1. **Request-Task**

    In `Request-Tasks` only the input-data are provided for the network in order to generate a output
    of the previouly trained network. The output is stored in `Shiori` as `Request-Result`.

### Checkpoint

`Checkpoints` are the serializied version of a `Cluster`. The Cluster will be converted into one
single blob, written to disc and registered in the database.

### Cluster-Template

`Cluster-Templates` are a custom-formated string, which defines the structure of the `Cluster`.
Basically it describes the sizes and order of the hexagons. See the docu of the
[cluster-templates](/user/cluster_templates/cluster_template).
