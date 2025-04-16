#TODO docstring
import json
import os
from datalayer.node.hash_node import HashNode
from datalayer.hash_algorithm.hash_algorithm import HashAlgorithm


class NinjaHashNode(HashNode):
    def __init__(self, id, hash_algorithm: HashAlgorithm, binary_name, architecture, function_name):
        super().__init__(id, hash_algorithm)
        self._name = binary_name
        self._architecture = architecture
        self._function_name = function_name

    def get_name(self):
        return self._name
    
    def get_size(self):
        return self._size
    
    def get_category(self):
        return self._category
    
    def get_file(self):
        return self._file
    
    def get_family_name(self):
        return self._family_name
    
    def get_draw_features(self):
        return {"names": {self._id: self._name},
                "architectures": {self._id: self._architecture},
                "functions": {self._id: self._function_name}
                }

    def is_equal(self, other):
        return self._name == other._name
    
    def __str__(self):
        return f"Binary: {self._name}"

