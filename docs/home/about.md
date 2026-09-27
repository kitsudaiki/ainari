## About

This project contains an entire IaaS (Infrastructure-as-a-Service) stack to manage virtual machines
and networks between them. It is focused on security and easy handling for users, admins and
developers, not on scalability. It is entirely a spare-time project of mine, without a bigger
purpose I need it for. At work I have and had a lot to do with the IaaS Openstack in many ways:
administration, deployment and even modifications of the code. At my time in the company SecuStack,
which was closed years ago, we developed and implemented security features directly into the
existing code of Openstack. I saw many bad things in the handling and the code and the Ainari
project here became basically a platform, where I currently implement my own vision of an IaaS, by
implementing my own architecture, avoiding every pain point I had with Openstack, implementing
security features we had or at least planned for Openstack back then, and implementing my own
feature ideas. See [feature overview](/home/features/).

Originally the project started years ago with an experimental neural network in C++. Later it was
moved to the server side with a REST-API. Then came a user-management, key-management, installation
automation, dashboard and so on. Over time, this effectively turned it into a
Neural-Network-as-a-Service. Not because I needed it, but because I like programming and I was
simply interested in implementing features into a project freely. In 2025 I manually refactored the
entire C++ code base into Rust and the simple dashboard from plain JavaScript into Vue.js with
TypeScript. The problem was that the experimental neural network core was still an experimental
construction site, which worked well for small tests, but was never as good as hoped and was
rewritten nearly every year for new conceptual ideas. So the infrastructure had a much better
quality than the core and was way over the top for what the neural network was usable for. So in
late summer 2026 the experimental neural network core was pulled out of this repo here and moved
into the small side-project [Saki](https://github.com/kitsudaiki/saki), where it runs as a small
library, which can be included in Python scripts. To reuse the remaining infrastructure of this
project and give it a real productive purpose, Ainari was moved into the direction of an IaaS
project. Instead of managing neural networks, virtual machines are now managed and datasets were
replaced by disk-images. Big parts were already compatible with the new direction and required no or
only a bit of modification work for the new core function. Version 
[v0.11.1](https://github.com/kitsudaiki/ainari/tree/v0.11.1) is the last version with the old 
neural network core.

Until I started to push the project into this new direction, everything in the project, except some
tiny code snippets, was written by hand: backend, frontend, documentation and automation. Because I
ran into big problems while implementing the new desired network stack for the new IaaS approach
with eBPF, I was more or less forced into using AI coding tools. The low-level network stuff with
silently dropped packets in kernel space was horrible to debug. I have to admit that I was impressed
by how well it was debugged and fixed by AI and how much time and nerves it saved me. Since then, I
have been using AI more actively, to speed up the progress. I still fix many things myself, review
each generated piece of code and make manual modifications to it. In the end I have to admit that I
actually use more AI in this project now than I ever expected. But because the very big foundation
of the project was clearly structured and written by hand, and because I keep control over
everything generated, this project has nearly no technical debt. Whenever I see something to
refactor in this project, I do it, because I know I avoid a lot of pain if I fix it as soon as
possible.
