import networkx

graph = networkx.gnm_random_graph(1000, 10000)
with open("scc.dat", "w") as file:
    file.write("start;\n")

    for edge in graph.edges():
        file.write(f"insert Edge({edge[0]}, {edge[1]}),\n")

    file.write("commit dump_changes;\ntimestamp;\n")
