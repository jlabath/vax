#!/bin/bash

# get data from  https://data.ontario.ca/dataset/covid-19-vaccine-data-in-ontario
curl -o cases_by_vac_status.json https://data.ontario.ca/datastore/dump/eed63cf2-83dd-4598-b337-b288c0a89a16?format=json
curl -o hosp_by_vac_status.json https://data.ontario.ca/datastore/dump/274b819c-5d69-4539-a4db-f2950794138c?format=json

