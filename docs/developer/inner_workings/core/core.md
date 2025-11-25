# Core

This is a short overview of the current state of the core-function. The concept is still under
development and permanently updated while working on the code of Ainari.

Reference implementation of the core functions, which are describe by this chapter:

- [Processing](https://github.com/kitsudaiki/ainari/blob/develop/src/sakura/src/core/processing/cpu/core_processing.h)
- [Backpropagation](https://github.com/kitsudaiki/ainari/blob/develop/src/sakura/src/core/processing/cpu/core_backpropagation.h)
- [Output-Processing](https://github.com/kitsudaiki/ainari/blob/develop/src/sakura/src/core/processing/cpu/output_processing.h)
- [Output-Backpropagation](https://github.com/kitsudaiki/ainari/blob/develop/src/sakura/src/core/processing/cpu/output_backpropagation.h)

## Basic architecture

!!! note

    I'm not really creative with names. If you have some better naming ideas for the components here,
    then please open an issue and let me know your suggestion.

A artificial neural network in this project is called `cluster`, which consist of multiple
`hexagons`:

![Cluster](cluster.drawio)

There are different types of hexagons: input, output and internal (hidden). At the beginning, after
creating a new cluster, there are no neurons (nodes) or connections between the neurons defined.
Everything is created at runtime while training the cluster. While this happens, the hexagons are
working like a plant trellis for the connections. The hexagon-structure was introduced as based for
the experimental [layer-less](/inner_workings/core/core/#no-strict-layer-structure) structure.

!!! info

    Theoretically there are multiple input- and output-hexagons possible within a cluster at any
    location, but this wasn't practically tested so far.

There are two types of training:

1. backpropagaion

    Like done in all classical artificial neurons networks the training is still primary done by
    backpropagation from the outputs to the inputs.

1. try-and-error

    The basic idea, which started this concept and was the only learning process in the first PoC's,
    was the principle of try-and-error. Simple creating random connections and destroy any
    connection again, which doesn't benefit the desired output. The training with only process was
    quite slow had a massive lack an accuracy, but basically it worked. At the moment this process
    is more a side feature in form of the [Reduction](/inner_workings/core/core/#reduction), which
    is less powerful than in the first PoC's. It will be further evaluated and improved again, when
    testing with bigger test-cases.

### Data-Structure

The data-structure within the code for the encapsulation looks like below:

![Workflow](objects.drawio)

There are 3 types of objects:

- **neurons**

    Neurons containing primary the collected incoming potential of the neuron. In case of the
    input-layer this potential is the same like the incoming value. In case if the neurons of the
    internal hexagons these is the collected input from all attached synapses.

    Beside the potential they also contain the refraction-time and cool-down, which is used for the
    experimental feature for
    [spikin neural networks](/inner_workings/core/core/#spiking-neural-network)

    _128 neurons = 1 neuron-block_

- **synapses**

    Each synapse is connected to a random neuron within the related neuron-block. Synapses are
    triggered by the potential of the source neurons. For this each synapse contains a
    border-values, which is used to identify, if the potential of the neuron is high enough to
    trigger the synapse or not. For the case that the synapse is triggered, it contains a weight,
    which is applied to the connection neuron. In case of the backpropagation this weight is
    adjusted. The border of the synapse is not affected by the backpropagation.

    _128 synapses = 1 synapse-section_

    _512 synapse-section = 1 synapse-block_

- **connections**

    These are primary for optimization by reducing the memory access, which is very critical in case
    of the CPU and RAM. 1 connection connect 1 neuron and 1 synapse-section with each other. Instead
    of iterating over all synapses, the iteration is done on the connections, because 1 connection
    is much smaller in memory-size than 1 synapse-section. Beside this the connections are stored
    separately with its own list. So when 1 connection-block is loaded from the RAM by the CPU and a
    whole page of 4KiB is loaded, than it loads automatically the next few coming connection-blocks
    as well.

    They also contain a lower bound and a potential-range, which is necessary for the activation of
    the related synapse-section.

    _512 connections = 1 connection-block_

In total the ratio between neuron-blocks, synapse-blocks and connection-blocks is **1:1:1**

!!! note

    The hard-coded numbers 128 and 512 were selected because of the tests on the CPU because of the
    block-sized in CUDA. Maybe they will be changed in the future again.

The following shows a minimal example for a cluster with these 3 object-types:

![Workflow](information_processing2.drawio)

The gray boxes on the left is the input-buffer and on the right side is the output-buffer. The other
boxes are the same synapse-, neuron- and connection-blocks like in the previous graphics. Each
hexagon of the cluster contains a list of neuron-blocks and connection-blocks and links to the
synapse-blocks, which are stored in a global buffer. The synapse-blocks are the bigges objects. So
storing them within a global buffer allows it, that all clusters of all users on the host are
sharing the memory equaly, to optimize the memory consumption. This global buffer is also
pre-allocated memory for more performance. The blocks are generated over time within the hexagon
like written in the [growing-process](/inner_workings/core/core/#growing-process). Each connection
of the connection-blocks is linked to any neuron within another hexagon and never within the same
hexagon. This graphics only show a simple linear version, but the same way the connections can link
in multiple dimensions if the hexagon-base-structure provide this. The output, on the right side of
this example, need some additional handling like written in the section for the
[output-processing](/inner_workings/core/core/#output) below.

### Hexagon-processing

In case of an input value greater 0.0 or a neuron with a positive potential, a connected
synapse-section is triggered.

![Workflow](example_section.drawio)

Each synapse within the section has a border-value. While the input runs though the section in
serial order, the potential will be reduced by the border of each passed synapse (red) until is
reach zero. Each synapse, which comes later in the section, will not be triggered (blue). This way,
different potentials triggering different amount of synapses. In case of the MNIST-dataset for
example this means, that a `1` as input need less processing power, than a `8`, because the `1`
always has less positive input-values to represent the image of the number. Also this is necessary
for the experimental feature of the
[spiking neural networks](/inner_workings/core/core/#spiking-neural-network), because different
timeframes of the spike and it cool-down also means a different amount of active synapse.

![Workflow](information_processing.drawio)

A neuron can be connected to multiple synapse-sections. The connection objects for each
synapse-section has a pointer to the source neuron. That way there is theoretically no limit in the
amount of synapses per neuron. Each connection has a lower bound and potential-range. In case the
lower-bound is higher than the potential of the potential of the linked source-neuron, the
synapse-section is not processed and not even loaded from the memory. The potential-range is
nessecary to identify how much the section can grow before splitting while
[growing](/inner_workings/core/core/#growing-process).

!!! info

    This functions is from one of the early stages, where also the input-hexagons used them while being
    only able to handle input of 0.0-1.0. Maybe is is unnecessary complicated now, so when testing with
    bigger test-cases, a simple boring linear function will also evaluated again.

### Growing process

The main-part of the grow-process happens within the synapse-section like shown as example below:

![Example](example_growing.drawio)

Each synapse has an activation border shown in the blue boxes. This is the representation within the
memory. The line is simple the logical relation of the values to each other and the dashed ovals,
even if they are not absolute exact in the graphics, are the tolerance of the synapses. The bigger
the tolerance is, the harder it is to create new synapses. The more synapses are created, the bigger
they become and the harder it becomes to create new one. This is primary done to limit the
growth-rate.

When new synapses are created, the weight is set randomly with a value smaller than 1.0 and random
sign.

!!! note

    The parameter of the change of the tolerance while learning is still hard-coded and is maybe not
    optimal for all use-cases. Need more evaluation of make it configurable, if no optimal solution was
    found.

When an active neuron triggers a synapse-section, 5 cases what can happen, while the potential is
applied to the synapse-section:

1. **The input match exactly to values of the section:**

    ![Example](example_growing1.drawio)

    :octicons-arrow-right-24: nothing is changed on the section

    <br>

1. **The input doesn't match exactly, but is still within the tolerance-range (dashed ovals) of a
    synapse**

    ![Example](example_growing2.drawio)

    :octicons-arrow-right-24: same like in case 1 that there are no changes

    <br>

1. **The section is NOT full with synapses and the input is higher than all synapse-borders
    together**

    ![Example](example_growing3.drawio)

    :octicons-arrow-right-24: a new synapse is created right at the end and get the remaining
    potential value as its border

    <br>

1. **The input doesn't match exactly, and is outside of the tolerance-range (dashed ovals) of a
    synapse**

    ![Example](example_growing4.drawio)

    - a new synapse is created right at the end and get as border

    <br>

1. **The section IS FULL with synapses and the input is higher than all synapse-borders together**

    ![Example](example_growing5.drawio)

    1. a new synapse-section is created in a random hexagon within a random block and linked by the
        connection to the same source-neuron, like the current processed section
    1. the counter, which counts the border-values of the first half of all synapses within the
        section (value 2.3 in this example) becomes the protental-range of the current section and
        the lower bound of the new section

    The new section will be only triggered by potentials of the neuron greater than the lower bound.
    Setting the potential-range of the current section to the same value limit the maximum
    potential processed by the section. That way where one section ends, the other one begins to
    avoid an overlap keep the resize more under control. Because of the limitation of the new
    potentatial-range of the current section, the second half of the section becomes basically
    erased. Thanks to the rules 3. and 4. the section can be refilled over time, based on the given
    input. In case the section becomes full again, step 5 is done the same way again.

    <br>

In case of case 5 new sections are created. The new sections are places in a random hexagon (based
on allowed hexagon for the current hexagon) and within a random synapse-block of this
target-hexagon. In case the synapse-blocks of the target-hexagon are in total already filled to a
certain level, a new neuron-, connection- and synapse-block is created for the hexagon.

Inputs and outputs can be simply resized based on the given input. Adding them in later iterations,
the new inputs and outputs doesn't automatically break the already trained data of the cluster,
because they have no connections when they are added, only when they are used while training.

### Output

The output layer is very classical like used in every classical artificial neural network.

![Workflow](output_processing.drawio)

The single outputs are connected with multiple neurons of the related output-hexagon without order.
Additionally they normalize the outgoing values to the range of 0.0-1.0 with the help of the
classical sigmoid function:

![Workflow](output_segment_function.jpg)

![Workflow](output_segment_graph.jpg)

The special point here is, that these are also not connected to the neurons of the output-hexagons
from the birth of the cluster. They connect at runtime randomly to neurons, which have a potential
greater than 0. So neurons in the output-hexagon, which never have a potential greater than 0 are
also never used by any output. Beside this there is only a limited number of connections per output
possible. So an output-value is never connected to all neurons of the related output-hexagon.
Because of this, resizing the number of output-values doesn't result in an exponential growth of the
connections to the neurons of the output-hexagon.

### Parallelism

The hexagons itself have to be processed one after another in a sequential order. Only the block
within the hexagons are processing in parallel.

![multi-threading](multithreading.drawio)

As soon as a new hexagon is started, all blocks of the hexagons are linked into a task-queue from
where they can be taken by the worker-threads in order to process them. Because the blocks can have
different amounts of active synapses, the processing-time per block will differ. So by using a
central queue instead of assigning them right from the beginning, there is an automatic
load-balancing. A thread, which has finished his current block, takes the next one from the queue,
until all blocks of the current hexagon are processed. All blocks of all clusters of all users are
using the same queue and worker-threads to optimize the load-balancing even more.

To make this parallel processing possible, it is necessary that connections are always linked to
neurons within another hexagon, to avoid race-conditions.

## Additional features

There are some even more experimental optional features, which can be enabled. They can be defined
in the [cluster-templates](/frontend/cluster_templates/cluster_template/). There are also a few
[measurement-examples](/inner_workings/measurements/measurements).

### No strict layer structure

The base of a new neural network is defined by a cluster-template. In these templates the structure
of the network in planed in hexagons, indeed of layer. When a node tries to create a new synapse,
the location of the target-node depends on the location of the source-node within these hexagons.
The target is random and the probability depends on the distance to the source. This way it is
possible to break the static layer structure.

![Connection-distance](connection_distance.drawio)

The orange hexagon is the source of the connection. It can be configured, how far a connection can
reach within the hexagon-structure. The red circle show a maximum distance of 1. So all neurons
within the orange hexagon can only connect to neurons within the hexagon within the red circle. The
green circle represents a maximum distance of 2. So the possible target of a new connection can be
anywhere within the green or red circle.

So for a simple lined sequence of hexagons with a maximum distance of **3** it could look like this:

![Connection-distance](connection_distance2.drawio)

The orange hexagon is the input and the red is the output and the arrows between them are showing
some possible connections, which can appear while learning.

Hexagons, which are near to the source hexagon, have a higher change to become the target, than a
hexagon far away.

A simple path finding process while initializing the structure, search a traces from input- to
output-hexagons. This way it should be prevented an uncontrolled structure and avoid cycles which
the network growth.

### Spiking neural network

This is a feature without a specific use-case in mind and only inspired by the human brain.

It consist of two parameter:

1. Cool-down of the neuron:

    Its the value of how much the potential of a neuron is reduced from one cycle to the next. As
    default it is so high, that the remaining potential has no impact on the next cycle anymore.

1. Refractory-Time:

    Gives the number of cycles until a triggered neuron can be triggered again by the input. Within
    this time-period, only the cool-down of the neuron is active.

The following shows the behavior of a neuron with a constant input of 100, a cooldown of 1,5 and a
refractory time of 3 :

![Spiking neuron](spiking_potential_cooldown1_5_refractory_3.jpg)
